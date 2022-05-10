// ----------------------------------------------------------------------------
use bevy::prelude::Color;
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct TimeOfDay {
    hour: u8,
    min: u8,
    sec: u8,
    caption: String,
}
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct ScalarCurveEntry(TimeOfDay, f32);
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct ColorCurveEntry(TimeOfDay, Color, f32);
// ----------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct Angle(u16);
// ----------------------------------------------------------------------------
impl TimeOfDay {
    // ------------------------------------------------------------------------
    pub fn new(hours: u8, minutes: u8, seconds: u8) -> Self {
        let hour = hours.min(23);
        let min = minutes.min(59);
        let sec = seconds.min(59);
        Self {
            caption: Self::fmt(hour, min),
            hour,
            sec,
            min,
        }
    }
    // ------------------------------------------------------------------------
    pub fn update(&mut self, linear: f32) {
        let t = linear.max(0.0) % 1.0 * (24 * 3600) as f32;
        let hour = (t / 3600.0).floor().min(23.0) as u8;
        let min = ((t / 60.0) % 60.0).floor().min(59.0) as u8;
        let sec = (t % 60.0).floor() as u8;

        if self.hour != hour || self.min != min || self.sec != sec {
            self.hour = hour;
            self.min = min;
            self.sec = sec;
            self.caption = Self::fmt(hour, min);
        }
    }
    // ------------------------------------------------------------------------
    /// [0..1.0]
    #[inline(always)]
    pub fn normalized(&self) -> f32 {
        Self::to_linear(self.hour, self.min, self.sec) / (24 * 3600) as f32
    }
    // ------------------------------------------------------------------------
    pub fn as_str(&self) -> &str {
        &self.caption
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn to_radians(&self) -> f32 {
        self.normalized() * 2.0 * std::f32::consts::PI
    }
    // ------------------------------------------------------------------------
    fn to_linear(hour: u8, min: u8, sec: u8) -> f32 {
        (hour as u32 * 3600 + min as u32 * 60 + sec as u32) as f32
    }
    // ------------------------------------------------------------------------
    fn fmt(hour: u8, min: u8) -> String {
        format!("{:0>2}:{:0>2}", hour, min)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ScalarCurveEntry {
    // ------------------------------------------------------------------------
    pub fn time(&self) -> &TimeOfDay {
        &self.0
    }
    // ------------------------------------------------------------------------
    pub fn value(&self) -> f32 {
        self.1
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ColorCurveEntry {
    // ------------------------------------------------------------------------
    pub fn time(&self) -> &TimeOfDay {
        &self.0
    }
    // ------------------------------------------------------------------------
    pub fn color(&self) -> &Color {
        &self.1
    }
    // ------------------------------------------------------------------------
    pub fn intensity(&self) -> f32 {
        self.2
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Angle {
    // ------------------------------------------------------------------------
    pub fn new(value: u16) -> Self {
        Self(value)
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn value(&self) -> u16 {
        self.0
    }
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn as_radians(&self) -> f32 {
        self.0 as f32 / 360.0 * 2.0 * std::f32::consts::PI
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl<'a> TryFrom<&'a str> for TimeOfDay {
    type Error = String;
    // ------------------------------------------------------------------------
    fn try_from(src: &str) -> Result<Self, String> {
        if src.len() == 5 {
            let (hour, minutes) = src.split_at(2);
            let hour: u8 = hour
                .parse()
                .map_err(|_| "could not parse hour".to_string())?;
            let minutes: u8 = minutes
                .split_at(1)
                .1
                .parse()
                .map_err(|_| "could not parse minutes".to_string())?;

            if (hour < 24 && minutes < 60) || (hour == 24 && minutes == 0) {
                return Ok(TimeOfDay::new(hour, minutes, 0));
            }
        }
        Err(format!("expected HH:mm. found: {}", src))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TryFrom<(&str, f32)> for ScalarCurveEntry {
    type Error = String;
    // ------------------------------------------------------------------------
    fn try_from(value: (&str, f32)) -> Result<Self, Self::Error> {
        let (time, value) = value;
        Ok(ScalarCurveEntry(TimeOfDay::try_from(time)?, value))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TryFrom<(&str, f32, f32, f32, f32)> for ColorCurveEntry {
    type Error = String;
    // ------------------------------------------------------------------------
    fn try_from(value: (&str, f32, f32, f32, f32)) -> Result<Self, Self::Error> {
        let (time, r, g, b, intensity) = value;
        Ok(ColorCurveEntry(
            TimeOfDay::try_from(time)?,
            Color::rgb(r / 255.0, g / 255.0, b / 255.0),
            intensity,
        ))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
