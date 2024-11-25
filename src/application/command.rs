#[derive(Debug)]
pub enum SCPCommand {
    Nominate,
    Hello,
}

impl SCPCommand {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "nominate" => Some(SCPCommand::Nominate),
            "hello" => Some(SCPCommand::Hello),
            _ => None,
        }
    }
}