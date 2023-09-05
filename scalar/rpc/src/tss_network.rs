use tracing::{debug, error, info, instrument, warn};

use crate::MessageOut;

#[derive(Clone)]
pub struct AnemoDeliverer {}
impl AnemoDeliverer {
    pub fn deliver(&self, msg: &MessageOut, from: &str) {
        info!("Deliver tss MessageOut {:?} from {:?}", msg, from);
    }
}
