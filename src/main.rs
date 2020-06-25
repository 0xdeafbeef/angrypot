mod data_collector;
mod server;

use crate::server::*;
use data_collector::{Collector, DbLogTypes};
use fern::colors::{Color, ColoredLevelConfig};
use log::trace;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use futures::try_join;
fn set_up_logging(level: u64) {
    // configure colors for the whole line
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        // we actually don't need to specify the color for debug and info, they are white by default
        .info(Color::White)
        .debug(Color::White)
        // depending on the terminals color scheme, this is the same as the background color
        .trace(Color::BrightBlack);

    // configure colors for the name of the level.
    // since almost all of them are the some as the color for the whole line, we
    // just clone `colors_line` and overwrite our changes
    let colors_level = colors_line.clone().info(Color::Green);
    // here we set up our fern Dispatch
    let verbosity = match level {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{target}][{level}{color_line}] {message}\x1B[0m",
                color_line = format_args!(
                    "\x1B[{}m",
                    colors_line.get_color(&record.level()).to_fg_str()
                ),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                target = record.target(),
                level = colors_level.color(record.level()),
                message = message,
            ));
        })
        // set the default log level. to filter out verbose log messages from dependencies, set
        // this to Warn and overwrite the log level for your crate.
        .level(verbosity)
        // change log levels for individual modules. Note: This looks for the record's target
        // field which defaults to the module path but can be overwritten with the `target`
        // parameter:
        // `info!(target="special_target", "This log message is about special_target");`
        .level_for("pretty_colored", log::LevelFilter::Trace)
        // output to stdout
        .chain(std::io::stdout())
        .apply()
        .expect("Failed setting up logging");
    trace!("finished setting up logging! yay!");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    set_up_logging(3);
    let mut config = thrussh::server::Config::default();
    config.connection_timeout = Some(std::time::Duration::from_secs(30));
    config.auth_rejection_time = std::time::Duration::from_secs(1);
    config
        .keys
        .push(thrussh_keys::key::KeyPair::generate_ed25519().unwrap());
    let config = Arc::new(config);
    let (tx, rx): (Sender<DbLogTypes>, Receiver<DbLogTypes>) = channel(100);
    let mut col = Collector::new(rx).await?;
    let sh = Server {
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
        tx,
    };
    let listener = col.run();
    let server_run = thrussh::server::run(config, "0.0.0.0:2222", sh);
    try_join!(server_run, listener).unwrap();
    dbg!("running");
    Ok(())
}
