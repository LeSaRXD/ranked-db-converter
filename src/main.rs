#![deny(clippy::unwrap_used)]

mod cli;

use std::{
	collections::HashSet,
	fs::File,
	io::{BufRead, BufReader},
	sync::Arc,
};

use clap::Parser;
use itertools::multiunzip;
use mcsr_ranked_api::{game::AdvancedMatchInfo, user::UserProfile};
use sqlx::{PgPool, types::chrono::NaiveDateTime};
use tokio::{sync::RwLock, task::JoinSet};
use uuid::Uuid;

use crate::cli::Cli;

// Tweak these to fit your hardware
const CHUNK_SIZE: usize = 1024;
const NUM_CHUNKS: usize = 16;

fn read_chunk<I>(lines: &mut I) -> Vec<AdvancedMatchInfo>
where
	I: Iterator<Item = AdvancedMatchInfo>,
{
	let mut chunk = Vec::with_capacity(CHUNK_SIZE);
	let mut count = CHUNK_SIZE;
	while let Some(info) = lines.next()
		&& count > 0
	{
		chunk.push(info);
		count -= 1;
	}
	chunk
}

#[tokio::main]
async fn main() {
	let cli = Cli::parse();
	assert!(
		cli.before > cli.after,
		"`before` cannot be less than or equal to `after`"
	);

	const DEFAULT_PATH: &str = "./dump.jsonl";
	let try_dump_file = match &cli.path {
		Some(p) => File::open(p),
		None => File::open(DEFAULT_PATH),
	};
	let dump_file = match try_dump_file {
		Ok(f) => BufReader::new(f),
		Err(e) => {
			eprintln!("Could not open dump file!\n{e:?}");
			return;
		}
	};

	let db_pool = connect_db().await;

	let mut games = dump_file
		.lines()
		.flat_map(
			|res| match res.map(|l| serde_json::from_str::<AdvancedMatchInfo>(&l)) {
				Ok(Ok(t)) => Some(t),
				Ok(Err(e)) => {
					eprintln!("Could not read line\n{e:?}");
					None
				}
				Err(e) => {
					eprintln!("Could not convert line to MatchInfo\n{e:?}");
					None
				}
			},
		)
		.skip_while(|m| m.info.id <= cli.after)
		.take_while(|m| m.info.id < cli.before);

	let chunks: Vec<_> = (0..NUM_CHUNKS).map(|_| read_chunk(&mut games)).collect();

	let mut inserted_games = 0;
	let mut tasks = JoinSet::new();
	for ch in chunks {
		tasks.spawn(process_games(ch, Arc::clone(&db_pool)));
	}
	while let Some(finished) = tasks.join_next().await {
		match finished {
			Ok(inserted) => inserted_games += inserted,
			Err(e) => eprintln!("Could not process chunk!\n{e:?}"),
		}
		let chunk = read_chunk(&mut games);
		if chunk.is_empty() {
			break;
		} else {
			tasks.spawn(process_games(chunk, Arc::clone(&db_pool)));
		}
	}
	inserted_games += tasks.join_all().await.iter().sum::<usize>();

	println!("Successfully inserted {inserted_games} matches into the database!");

	post_convert(db_pool).await;
}

async fn connect_db() -> Arc<RwLock<PgPool>> {
	let url = dotenvy::var("DATABASE_URL").expect("No DATABASE_URL in env");
	Arc::new(RwLock::new(
		PgPool::connect(&url)
			.await
			.expect("Could not connect to the database"),
	))
}

async fn process_games(games: Vec<AdvancedMatchInfo>, pool: Arc<RwLock<PgPool>>) -> usize {
	let players: HashSet<_> = games
		.iter()
		.flat_map(|g| {
			g.info
				.players()
				.iter()
				.map(convert_player)
				.collect::<Vec<_>>()
		})
		.collect();
	let (player_ids, player_usernames): (Vec<_>, Vec<_>) = players.into_iter().unzip();

	let pool = &*pool.write().await;
	sqlx::query!(
		r#"INSERT INTO player
		(id, username)
		SELECT
		UNNEST($1::UUID[]), UNNEST($2::VARCHAR[])
		ON CONFLICT (id) DO NOTHING"#,
		&player_ids,
		&player_usernames as _,
	)
	.execute(pool)
	.await
	.expect("Could not write players to database");

	let (ids, kinds, seasons, dates, winner_ids, times, forfeits, decays, replays): (
		Vec<_>,
		Vec<_>,
		Vec<_>,
		Vec<_>,
		Vec<_>,
		Vec<_>,
		Vec<_>,
		Vec<_>,
		Vec<_>,
	) = multiunzip(games.iter().map(|g| {
		(
			g.info.id as i64,
			g.info.kind as i16,
			g.info.season as i16,
			NaiveDateTime::new(g.info.date.date_naive(), g.info.date.time()),
			g.info.result.winner_uuid,
			g.info.result.time.0 as i64,
			g.info.forfeited,
			g.info.decayed,
			g.replay_exists,
		)
	}));

	sqlx::query!(
		r#"INSERT INTO game
		(id, kind, season, date, winner_id, time, forfeited, decayed, replay_exists)
		SELECT * FROM (
			SELECT
				UNNEST($1::BIGINT[]),
				UNNEST($2::SMALLINT[]),
				UNNEST($3::SMALLINT[]),
				UNNEST($4::TIMESTAMP[]),
				UNNEST($5::UUID[]) as winner_id,
				UNNEST($6::BIGINT[]),
				UNNEST($7::BOOLEAN[]),
				UNNEST($8::BOOLEAN[]),
				UNNEST($9::BOOLEAN[])
			)
		WHERE EXISTS(SELECT * FROM player WHERE id = winner_id)
		ON CONFLICT (id)
		DO UPDATE SET
			kind = EXCLUDED.kind,
			season = EXCLUDED.season,
			date = EXCLUDED.date,
			winner_id = EXCLUDED.winner_id,
			time = EXCLUDED.time,
			forfeited = EXCLUDED.forfeited,
			decayed = EXCLUDED.decayed,
			replay_exists = EXCLUDED.replay_exists"#,
		&ids,
		&kinds,
		&seasons,
		&dates,
		&winner_ids as _,
		&times,
		&forfeits,
		&decays,
		&replays,
	)
	.execute(pool)
	.await
	.expect("Could not write games to database");

	let (game_ids, player_ids, elo_changes, new_elos): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) =
		multiunzip(games.iter().flat_map(|g| {
			g.info.elo_updates.iter().map(|upd| {
				(
					g.info.id as i64,
					upd.player_uuid,
					upd.change,
					upd.elo.map(|e| e as i16),
				)
			})
		}));

	sqlx::query!(
		r#"INSERT INTO elo_change
		(game_id, player_id, change, old_elo)
		SELECT * FROM (
			SELECT
				UNNEST($1::BIGINT[]) as game_id,
				UNNEST($2::UUID[]),
				UNNEST($3::SMALLINT[]),
				UNNEST($4::SMALLINT[])
			)
		WHERE EXISTS(SELECT * FROM game where id = game_id)
		ON CONFLICT (game_id, player_id)
		DO UPDATE SET
		change = EXCLUDED.change"#,
		&game_ids,
		&player_ids,
		&elo_changes as _,
		&new_elos as _,
	)
	.execute(pool)
	.await
	.expect("Could not write elo changes to database");

	games.len()
}

fn convert_player(info: &UserProfile) -> (Uuid, &str) {
	(info.uuid, &info.nickname)
}

async fn post_convert(pool: Arc<RwLock<PgPool>>) {
	let pool = &*pool.write().await;
	sqlx::query_file!("./update_elo.sql")
		.execute(pool)
		.await
		.expect("Could not update player elo");
}
