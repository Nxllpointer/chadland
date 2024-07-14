use chadland::Options;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    chadland::run(Options {})
}
