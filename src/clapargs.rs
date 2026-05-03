use clap::Parser;

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "haruhishot",
    about="One day Haruhi Suzumiya made a wlr screenshot tool",
    long_about = None,
    version,
)]
pub enum HaruhiCli {
    #[command(
        long_flag = "list-outputs",
        short_flag = 'L',
        about = "list all outputs"
    )]
    ListOutputs,
    #[command(long_flag = "output", short_flag = 'O', about = "choose output")]
    Output {
        #[arg(required = false)]
        output: Option<String>,
        #[arg(value_name = "stdout", long)]
        stdout: bool,
        #[arg(value_name = "pointer", long, default_value = "false")]
        cursor: bool,
    },
    #[command(long_flag = "slurp", short_flag = 'S', about = "area select")]
    Slurp {
        #[arg(value_name = "stdout", long)]
        stdout: bool,
        #[arg(value_name = "pointer", long, default_value = "false")]
        cursor: bool,
    },
    #[command(long_flag = "application", about = "application shot")]
    Application {
        #[arg(value_name = "stdout", long)]
        stdout: bool,
        #[arg(value_name = "pointer", long, default_value = "false")]
        cursor: bool,
    },
    #[command(long_flag = "fullscreen", short_flag = 'F', about = "capture all outputs")]
    Fullscreen {
        #[arg(value_name = "stdout", long)]
        stdout: bool,
        #[arg(value_name = "pointer", long, default_value = "false")]
        cursor: bool,
    },
    #[command(long_flag = "color", short_flag = 'C', about = "get color")]
    Color,
    #[command(
        long_flag = "all-outputs",
        short_flag = 'A',
        about = "capture all outputs and concatenate horizontally"
    )]
    AllOutputs {
        #[arg(value_name = "stdout", long)]
        stdout: bool,
        #[arg(value_name = "pointer", long, default_value = "false")]
        cursor: bool,
    },
}
