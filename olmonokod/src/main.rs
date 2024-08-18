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
        host: String,
        email: String,
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
        Commands::Login { host, email } => {
            let password = rpassword::prompt_password("password: ").unwrap();

            let login_result = olmonoko::create_session(&host, &email, &password)
                .await
                .unwrap();
            if let Some(session_id) = login_result {
                println!("Login successful! Session created.");
                let mut keyring = get_os_keyring("olmonokod").expect("acquiring os keyring");
                keyring
                    .set_secret("host", &host)
                    .expect("saving host url into keyring");
                keyring
                    .set_secret("session_id", &session_id)
                    .expect("saving session id into keyring");

                println!("credentials saved");
            } else {
                println!("WAH WAH");
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
            let session_id = String::from_utf8(
                keyring
                    .get_secret("session_id")
                    .expect("getting session id from keyring")
                    .to_vec(),
            )
            .expect("decoding session_id from keyring");
            let result = olmonoko::get_user_details(&host, &session_id)
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
            let session_id = String::from_utf8(
                keyring
                    .get_secret("session_id")
                    .expect("getting session id from keyring")
                    .to_vec(),
            )
            .expect("decoding session_id from keyring");
            let result = olmonoko::get_upcoming_events(&host, &session_id)
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
