// ----------------------------------------------------------------------------
pub struct TerrainMeshSettings {
    /// deactivates reduction: current lods are frozen if anchor moves
    pub ignore_anchor: bool,
    /// number of lod levels (clamp(0, lod_count)). default 3
    pub lod_count: u8,
    /// minimum error threshold in meters for lod 0. default: 0.01 m
    pub min_error: f32,
    /// maximum error threshold in meters for lod n. default: 1.0 m
    pub max_error: f32,
    /// start distance for lod n
    pub max_distance: f32,
    /// precalculated error thresholds and debug info (e.g. start distance)
    lods: Vec<TerrainLodSettings>,
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodSlot(u8);
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub struct TerrainLodSettings {
    /// level for debugging
    pub level: u8,
    /// starting distance for lod in meters
    pub distance: f32,
    /// errormap threshold in meters
    pub threshold: f32,
}
// ----------------------------------------------------------------------------
impl TerrainLodSettings {
    // ------------------------------------------------------------------------
    fn new(level: u8, start_distance: f32, error_threshold: f32) -> Self {
        Self {
            level,
            distance: start_distance,
            threshold: error_threshold,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Default for TerrainMeshSettings {
    fn default() -> Self {
        let mut s = Self {
            ignore_anchor: false,
            lod_count: 3,
            min_error: 0.01,
            max_error: 1.0,
            max_distance: 1000.0,

            lods: Vec::default(),
        };
        s.update_lodsettings();
        s
    }
}
// ----------------------------------------------------------------------------
impl TerrainMeshSettings {
    // ------------------------------------------------------------------------
    pub fn setup_defaults_from_size(&mut self, mapsize: u32) {
        if mapsize > 4096 {
            self.lod_count = 6;
            self.min_error = 0.01;
            self.max_error = 5.0;
            self.max_distance = 4000.0;
            // error values are porbably more dependent on height range in map
            // than mapsize
            self.lods = vec![
                TerrainLodSettings::new(0, 0.0, 0.01),
                TerrainLodSettings::new(1, 250.0, 0.075),
                TerrainLodSettings::new(2, 500.0, 0.3),
                TerrainLodSettings::new(3, 1000.0, 0.5),
                TerrainLodSettings::new(4, 2000.0, 1.0),
                TerrainLodSettings::new(5, 3500.0, 1.5),
            ];
        } else if mapsize > 2048 {
            self.lod_count = 4;
            self.min_error = 0.01;
            self.max_error = 1.0;
            self.max_distance = 1000.0;
            self.lods = vec![
                TerrainLodSettings::new(0, 0.0, 0.01),
                TerrainLodSettings::new(1, 250.0, 0.05),
                TerrainLodSettings::new(2, 500.0, 0.25),
                TerrainLodSettings::new(3, 750.0, 0.5),
            ];
        } else {
            self.lod_count = 3;
            self.min_error = 0.01;
            self.max_error = 1.0;
            self.max_distance = 1000.0;
            self.lods = vec![
                TerrainLodSettings::new(0, 0.0, 0.1),
                TerrainLodSettings::new(1, 100.0, 0.5),
                TerrainLodSettings::new(2, 1000.0, 1.0),
            ];
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TerrainMeshSettings {
    // ------------------------------------------------------------------------
    pub fn lod_settings(&self) -> impl Iterator<Item = &TerrainLodSettings> {
        self.lods.iter()
    }
    // ------------------------------------------------------------------------
    pub fn set_lodcount(&mut self, count: u8) {
        self.lod_count = count.clamp(0, 10);
        self.update_lodsettings();
    }
    // ------------------------------------------------------------------------
    pub fn set_min_error(&mut self, threshold: f32) {
        self.min_error = threshold.clamp(0.001, 5.0);
        self.max_error = self.max_error.max(self.min_error);
        self.set_lod_error(LodSlot(0), self.min_error);
    }
    // ------------------------------------------------------------------------
    pub fn set_max_error(&mut self, threshold: f32) {
        self.max_error = threshold.clamp(self.min_error, 20.0);
        self.set_lod_error(LodSlot(self.lods.len() as u8), self.max_error);
    }
    // ------------------------------------------------------------------------
    pub fn set_max_distance(&mut self, distance: f32) {
        self.max_distance = distance.clamp(250.0, 10000.0);
        self.set_lod_distance(LodSlot(self.lods.len() as u8), self.max_distance);
    }
    // ------------------------------------------------------------------------
    pub fn set_lod_error(&mut self, lod: LodSlot, threshold: f32) {
        // clamp threshold and slot
        let threshold = self.min_error.max(threshold).min(self.max_error);
        let slot = lod.to_usize().min(self.lods.len().saturating_sub(1));

        // clamp all lower lods
        if slot > 0 {
            for lod in self.lods.iter_mut().take(slot) {
                lod.threshold = lod.threshold.min(threshold);
            }
        }

        if let Some(lod) = self.lods.get_mut(slot) {
            lod.threshold = threshold;
        }

        // clamp all higher lods
        if slot < self.lods.len() {
            for lod in self.lods.iter_mut().skip(slot + 1) {
                lod.threshold = lod.threshold.max(threshold);
            }
        }
    }
    // ------------------------------------------------------------------------
    pub fn set_lod_distance(&mut self, lod: LodSlot, distance: f32) {
        // lod 0 starts always at 0 distance
        if lod.0 > 0 {
            // clamp distance and slot
            let distance = self.max_distance.min(distance).max(0.0);
            let slot = lod.to_usize().min(self.lods.len().saturating_sub(1));

            // clamp all lower lods
            if slot > 0 {
                for lod in self.lods.iter_mut().take(slot) {
                    lod.distance = lod.distance.min(distance);
                }
            }

            if let Some(lod) = self.lods.get_mut(slot) {
                lod.distance = distance;
            }

            // clamp all higher lods
            if slot < self.lods.len() {
                for lod in self.lods.iter_mut().skip(slot + 1) {
                    lod.distance = lod.distance.max(distance);
                }
            }
        }
    }
    // ------------------------------------------------------------------------
    /// Adds or removes lod level. values for newly added lod levels are
    /// interpolated linearly from last existing level to max error and distance
    /// settings.
    fn update_lodsettings(&mut self) {
        let new_lod_count = self.lod_count.max(1) as usize;

        match self.lods.len() {
            0 => {
                // make sure first lod starts at min settings and if lod
                // count > 1 last level has max settings
                let new_lods = (new_lod_count - 1).max(1);
                self.lods = (1..=new_lod_count)
                    .map(|i| {
                        let step = (i - 1) as f32 / new_lods as f32;
                        TerrainLodSettings::new(
                            (i - 1) as u8,
                            self.max_distance * step,
                            self.min_error + (self.max_error - self.min_error) * step,
                        )
                    })
                    .collect::<Vec<_>>();
            }
            lod_count if lod_count < new_lod_count => {
                let new_lods = new_lod_count - self.lods.len();

                let last_lod = self.lods.last().unwrap();
                let last_lod_distance = last_lod.distance;
                let last_lod_threshold = last_lod.threshold;

                for i in 1..=new_lods {
                    let step = i as f32 / new_lods as f32;
                    self.lods.push(TerrainLodSettings::new(
                        self.lods.len() as u8,
                        last_lod_distance + (self.max_distance - last_lod_distance) * step,
                        last_lod_threshold + (self.max_error - last_lod_threshold) * step,
                    ));
                }
            }
            lod_count if lod_count > new_lod_count => {
                self.lods = self.lods.drain(..new_lod_count).collect();
            }
            _ => {}
        }
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub(super) fn lod_settings_from_distance(&self, distance: f32) -> &TerrainLodSettings {
        if distance > 0.0 {
            for prev_lod in self.lods.iter().rev() {
                if distance > prev_lod.distance {
                    return prev_lod;
                }
            }
        }
        &self.lods[0]
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl LodSlot {
    fn to_usize(self) -> usize {
        self.0 as usize
    }
}
// ----------------------------------------------------------------------------
impl From<u8> for LodSlot {
    fn from(v: u8) -> Self {
        LodSlot(v)
    }
}
// ----------------------------------------------------------------------------
