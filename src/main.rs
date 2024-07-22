fn main() {
    tracing_subscriber::fmt().init();

    chadland::run::<chadland::backends::winit::WinitBackend>();
}
