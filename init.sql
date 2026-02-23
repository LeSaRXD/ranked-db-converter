--
-- PostgreSQL database dump
--

\restrict rwqbVDsj5QT9FR2hYV6DoW6W7849AXyFQkDi3H5WK2zwvdvu9DFVb40m5mZxIGp

-- Dumped from database version 18.2
-- Dumped by pg_dump version 18.2

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: elo_change; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.elo_change (
    game_id bigint NOT NULL,
    player_id uuid NOT NULL,
    change smallint,
    new_elo smallint
);


--
-- Name: game; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.game (
    id bigint NOT NULL,
    kind smallint NOT NULL,
    season smallint NOT NULL,
    date timestamp without time zone NOT NULL,
    winner_id uuid,
    "time" bigint NOT NULL,
    forfeited boolean NOT NULL,
    decayed boolean NOT NULL,
    replay_exists boolean NOT NULL
);


--
-- Name: player; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.player (
    id uuid NOT NULL,
    username character varying(17) NOT NULL,
    elo smallint
);


--
-- Name: elo_change elo_change_game_id_player_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.elo_change
    ADD CONSTRAINT elo_change_game_id_player_id_key UNIQUE (game_id, player_id);


--
-- Name: game game_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.game
    ADD CONSTRAINT game_pkey PRIMARY KEY (id);


--
-- Name: player player_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.player
    ADD CONSTRAINT player_pkey PRIMARY KEY (id);


--
-- Name: elo_change elo_change_game_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.elo_change
    ADD CONSTRAINT elo_change_game_id_fkey FOREIGN KEY (game_id) REFERENCES public.game(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: elo_change elo_change_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.elo_change
    ADD CONSTRAINT elo_change_player_id_fkey FOREIGN KEY (player_id) REFERENCES public.player(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- Name: game game_winner_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.game
    ADD CONSTRAINT game_winner_id_fkey FOREIGN KEY (winner_id) REFERENCES public.player(id) ON UPDATE CASCADE ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict rwqbVDsj5QT9FR2hYV6DoW6W7849AXyFQkDi3H5WK2zwvdvu9DFVb40m5mZxIGp

