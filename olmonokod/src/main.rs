mod olmonoko;

use clap::{Parser, Subcommand};
use cryptex::{get_os_keyring, KeyRing};
use daemonize::Daemonize;

/// Daemon to communicate with the OLMONOKO calendar multiplexer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the daemon
    Daemon,

    /// Log In to an olmonoko server
    Login {
        /// Server url, eg. https://olmonoko.example.com
        host: String
    },

    /// Get user details, if logged in
    AuthStatus,

    /// Get upcoming events you are planning to attend
    Events,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Daemon => {
            let daemonize = Daemonize::new().pid_file("/tmp/olmonokod.pid");
            match daemonize.start() {
                Ok(_) => println!("Success, daemonized"),
                Err(e) => eprintln!("Error, {}", e),
            }
            println!("Hello, world!");
        }
        Commands::Login { host } => {
            let api_key = rpassword::prompt_password("API Key: ").unwrap();

            let user_details = olmonoko::get_user_details(&host, &api_key).await.unwrap();
            if let Some(user) = user_details {
                println!("API Key is valid. Hello {}!", user.email);
                let mut keyring = get_os_keyring("olmonokod").expect("acquiring os keyring");
                keyring
                    .set_secret("host", &host)
                    .expect("saving host url into keyring");
                keyring
                    .set_secret("api_key", &api_key)
                    .expect("saving api key into keyring");

                println!("credentials saved");
            } else {
                println!("Could not get user info?");
            }
        }
        Commands::AuthStatus => {
            let mut keyring = get_os_keyring("olmonokod").expect("acquiring os keyring");
            let host = String::from_utf8(
                keyring
                    .get_secret("host")
                    .expect("getting host url from keyring")
                    .to_vec(),
            )
            .expect("decoding host from keyring");
            let api_key = String::from_utf8(
                keyring
                    .get_secret("api_key")
                    .expect("getting api key from keyring")
                    .to_vec(),
            )
            .expect("decoding api key from keyring");
            let result = olmonoko::get_user_details(&host, &api_key)
                .await
                .unwrap();
            if let Some(details) = result {
                println!("{}", serde_json::to_string_pretty(&details).unwrap());
            } else {
                println!("WAH WAH");
            }
        }
        Commands::Events => {
            let mut keyring = get_os_keyring("olmonokod").expect("acquiring os keyring");
            let host = String::from_utf8(
                keyring
                    .get_secret("host")
                    .expect("getting host url from keyring")
                    .to_vec(),
            )
            .expect("decoding host from keyring");
            let api_key = String::from_utf8(
                keyring
                    .get_secret("api_key")
                    .expect("getting api key from keyring")
                    .to_vec(),
            )
            .expect("decoding api key from keyring");
            let result = olmonoko::get_upcoming_events(&host, &api_key)
                .await
                .unwrap();
            if let Some(details) = result {
                println!("{}", serde_json::to_string_pretty(&details).unwrap());
            } else {
                println!("WAH WAH");
            }
        }
    }
}
