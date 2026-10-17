use tracing_subscriber::EnvFilter;

const FILTER_VARIABLE: &str = "SURT_LOG";

pub(crate) fn install() {
    if std::env::var_os(FILTER_VARIABLE).is_none() {
        return;
    }
    let filter = EnvFilter::builder()
        .with_env_var(FILTER_VARIABLE)
        .from_env_lossy();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
