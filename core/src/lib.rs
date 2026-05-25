pub mod time;

#[derive(Clone, Copy, Debug)]
pub enum EngineEvent {
    Resize { width: u32, height: u32 },
    ScaleFactorChanged { scale_factor: f64 },
    Quit,
}
