use crate::websocket::SuperCommand;

#[derive(Clone, Debug)]
pub struct PingSuperCommand;
impl SuperCommand for PingSuperCommand {}
