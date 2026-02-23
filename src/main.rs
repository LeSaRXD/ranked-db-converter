use std::{
	collections::HashSet,
	fs::File,
	io::{BufRead, BufReader},
	sync::Arc,
};

use itertools::multiunzip;
use mcsr_ranked_api::{game::AdvancedMatchInfo, user::UserProfile};
use sqlx::{PgPool, types::chrono::NaiveDateTime};
use tokio::{sync::RwLock, task::JoinSet};
use uuid::Uuid;

#[tokio::main]
async fn main() {
	let path = std::env::args()
		.nth(1)
		.unwrap_or_else(|| "./dump.jsonl".to_owned());
	let dump_file = BufReader::new(File::open(path).expect("File ./dump.jsonl does not exist"));

	let db_pool = connect_db().await;

	let mut games = dump_file.lines();

	const CHUNK_SIZE: usize = 1024;

	let mut chunks = Vec::new();
	while let Some(Ok(first)) = games.next() {
		let mut chunk = Vec::with_capacity(CHUNK_SIZE);
		chunk.push(first);
		for _ in 1..CHUNK_SIZE {
			if let Some(Ok(next)) = games.next() {
				chunk.push(next);
			} else {
				break;
			}
		}
		chunks.push(chunk);
	}

	let mut tasks = JoinSet::new();
	for ch in chunks {
		tasks.spawn(process_games(ch, Arc::clone(&db_pool)));
	}
	tasks.join_all().await;

	post_convert(db_pool).await;
}

async fn connect_db() -> Arc<RwLock<PgPool>> {
	let url = dotenvy::var("DATABASE_URL").expect("No DATABASE_URL in env");
	Arc::new(RwLock::new(PgPool::connect(&url).await.unwrap()))
}

async fn process_games(games: Vec<String>, pool: Arc<RwLock<PgPool>>) {
	let games: Vec<AdvancedMatchInfo> = games
		.into_iter()
		.flat_map(|g| serde_json::from_str(&g).ok())
		.collect();
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
	.unwrap();

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
	.unwrap();

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
		(game_id, player_id, change, new_elo)
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
	.unwrap();
}

fn convert_player(info: &UserProfile) -> (Uuid, &str) {
	(info.uuid, &info.nickname)
}

async fn post_convert(pool: Arc<RwLock<PgPool>>) {
	let pool = &*pool.write().await;
	sqlx::query_file!("./update_elo.sql")
		.execute(pool)
		.await
		.unwrap();
}
