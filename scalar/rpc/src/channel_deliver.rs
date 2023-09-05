// #[derive(Clone)]
// pub struct Deliverer {
//     senders: HashMap<String, mpsc::UnboundedSender<MessageIn>>, // (party_uid, sender)
// }
// pub type SenderReceiver = (
//     mpsc::UnboundedSender<MessageIn>,
//     mpsc::UnboundedReceiver<MessageIn>,
// );
// impl Deliverer {
//     pub(super) fn with_party_ids(party_ids: &[String]) -> (Self, Vec<SenderReceiver>) {
//         let channels: Vec<SenderReceiver> = (0..party_ids.len())
//             .map(|_| mpsc::unbounded_channel())
//             .collect();
//         let senders = party_ids
//             .iter()
//             .cloned()
//             .zip(channels.iter().map(|(tx, _)| tx.clone()))
//             .collect();
//         (Deliverer { senders }, channels)
//     }
//     pub fn deliver(&self, msg: &MessageOut, from: &str) {
//         let msg = msg.data.as_ref().expect("missing data");
//         let msg = match msg {
//             message_out::Data::Traffic(t) => t,
//             _ => {
//                 panic!("msg must be traffic out");
//             }
//         };

//         // simulate wire transmission: translate proto::MessageOut to proto::MessageIn
//         let msg_in = MessageIn {
//             data: Some(message_in::Data::Traffic(TrafficIn {
//                 from_party_uid: from.to_string(),
//                 is_broadcast: msg.is_broadcast,
//                 payload: msg.payload.clone(),
//             })),
//         };

//         // deliver all msgs to all parties (even p2p msgs)
//         for (_, sender) in self.senders.iter() {
//             // we need to catch for errors in case the receiver's channel closes unexpectedly
//             if let Err(err) = sender.send(msg_in.clone()) {
//                 error!("Error in deliverer while sending message: {:?}", err);
//             }
//         }
//     }
//     pub fn send_timeouts(&self, secs: u64) {
//         let abort = message_in::Data::Abort(false);
//         let msg_in = MessageIn { data: Some(abort) };

//         // allow honest parties to exchange messages for this round
//         let t = std::time::Duration::from_secs(secs);
//         std::thread::sleep(t);

//         // deliver to all parties
//         for (_, sender) in self.senders.iter() {
//             sender.send(msg_in.clone()).unwrap();
//         }
//     }
// }
