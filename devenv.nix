{
  pkgs,
  # lib,
  # config,
  # inputs,
  ...
}:

{
  # https://devenv.sh/basics/
  env = {
    DATABASE_URL = "postgres://postgres:example@localhost:5488/olmonoko";
    SITE_URL = "http://localhost:8080";
  };
  dotenv.enable = true;

  # https://devenv.sh/packages/
  packages = [
    # Docker
    pkgs.docker
    pkgs.docker-compose

    # Rust
    # pkgs.rust-bin.nightly.latest.default
    # pkgs.cargo # Package manager
    pkgs.cargo-audit # Manual checks against security vulnerabilities
    pkgs.rustfmt # Formatting
    pkgs.bacon # Constant feedback
    pkgs.cargo-watch
    pkgs.clippy # Tips & tricks

    # SQL Helper
    pkgs.sqlx-cli
  ];

  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "stable";
  };

  # https://devenv.sh/processes/
  # processes.cargo-watch.exec = "cargo-watch";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  # scripts.hello.exec = ''
  #   echo hello from $GREET
  # '';

  enterShell = ''
    git --version
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
