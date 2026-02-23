WITH new AS (
	SELECT DISTINCT ON (player_id)
		player_id,
		new_elo + change as elo,
		date
	FROM elo_change, game
	WHERE game_id = game.id
	AND new_elo IS NOT NULL
	ORDER BY player_id, date desc
)
UPDATE player
SET elo = new.elo
FROM (SELECT * FROM new) AS new
WHERE id = new.player_id
