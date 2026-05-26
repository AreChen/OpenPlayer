use super::*;

#[cfg(windows)]
impl MpvWallState {
    pub(super) fn next_generation(&self) -> Result<u64, String> {
        let mut generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        *generation = generation.saturating_add(1);
        Ok(*generation)
    }

    pub(super) fn current_generation(&self) -> Result<u64, String> {
        let generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(*generation)
    }

    pub(super) fn is_generation_current(&self, expected: u64) -> Result<bool, String> {
        let generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(*generation == expected)
    }

    pub(super) fn can_reuse_open_wall(
        &self,
        tiles: &[NormalizedMpvWallTileRequest],
    ) -> Result<bool, String> {
        if tiles.is_empty() {
            return Ok(false);
        }
        let statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        if statuses.len() != tiles.len() {
            return Ok(false);
        }

        Ok(tiles.iter().all(|tile| {
            statuses
                .get(&tile.id)
                .is_some_and(|snapshot| snapshot.url == tile.url)
        }))
    }

    pub(super) fn replace_opening_state(
        &self,
        snapshots: Vec<MpvWallTileSnapshot>,
    ) -> Result<(), String> {
        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        starting.clear();
        drop(starting);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        *statuses = snapshots
            .into_iter()
            .map(|snapshot| (snapshot.id.clone(), snapshot))
            .collect();
        Ok(())
    }

    pub(super) fn take_players(&self) -> Result<BTreeMap<String, MpvWallPlayer>, String> {
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(std::mem::take(&mut *players))
    }

    pub(super) fn insert_player(
        &self,
        generation: u64,
        player: MpvWallPlayer,
        status: &str,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let snapshot = player.status_snapshot(status, None);
        let id = player.id.clone();
        let _ = self.clear_tile_starting(&id);
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        players.insert(id.clone(), player);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(id, snapshot);
        Ok(())
    }

    pub(super) fn mark_tile_starting(&self, generation: u64, id: &str) -> Result<bool, String> {
        if !self.is_generation_current(generation)? {
            return Ok(false);
        }
        let players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        if players.contains_key(id) {
            return Ok(false);
        }
        drop(players);

        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(starting.insert(id.to_string()))
    }

    pub(super) fn clear_tile_starting(&self, id: &str) -> Result<(), String> {
        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        starting.remove(id);
        Ok(())
    }

    pub(super) fn update_player_status(
        &self,
        generation: u64,
        id: &str,
        status: &str,
        message: Option<String>,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        let Some(player) = players.get(id) else {
            return Ok(());
        };
        let snapshot = player.status_snapshot(status, message);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(id.to_string(), snapshot);
        Ok(())
    }

    pub(super) fn update_tile_error(
        &self,
        generation: u64,
        tile: &NormalizedMpvWallTileRequest,
        message: String,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let _ = self.clear_tile_starting(&tile.id);
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        players.remove(&tile.id);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(
            tile.id.clone(),
            wall_tile_status_snapshot(tile, "error", Some(message)),
        );
        Ok(())
    }
}
