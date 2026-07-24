//! Synchronous inter-process communication for ARIA.

use std::collections::{BTreeMap, VecDeque};
use std::time::SystemTime;

/// Uniquely identifies a registered process.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProcessId(String);

impl ProcessId {
    /// Creates a process identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the process identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProcessId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ProcessId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Uniquely identifies an IPC message within a message bus.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MessageId(u64);

impl MessageId {
    /// Creates a message identifier.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the numeric message identifier.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// An immutable IPC message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    id: MessageId,
    sender: ProcessId,
    receiver: ProcessId,
    timestamp: SystemTime,
    payload: Vec<u8>,
    reply_to: Option<MessageId>,
}

impl Message {
    fn new(
        id: MessageId,
        sender: ProcessId,
        receiver: ProcessId,
        payload: Vec<u8>,
        reply_to: Option<MessageId>,
    ) -> Self {
        Self {
            id,
            sender,
            receiver,
            timestamp: SystemTime::now(),
            payload,
            reply_to,
        }
    }

    /// Returns the message's unique identifier.
    pub fn id(&self) -> MessageId {
        self.id
    }

    /// Returns the sending process.
    pub fn sender(&self) -> &ProcessId {
        &self.sender
    }

    /// Returns the receiving process.
    pub fn receiver(&self) -> &ProcessId {
        &self.receiver
    }

    /// Returns the message creation timestamp.
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    /// Returns the message payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Returns the request message this message replies to, if any.
    pub fn reply_to(&self) -> Option<MessageId> {
        self.reply_to
    }
}

/// Errors returned by IPC operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IpcError {
    /// The sender process is not registered.
    SenderNotRegistered(ProcessId),
    /// The receiver process is not registered.
    ReceiverNotRegistered(ProcessId),
    /// The request message could not be found for reply correlation.
    RequestNotFound(MessageId),
}

/// FIFO message channel for a single process.
#[derive(Default)]
pub struct Channel {
    pending: VecDeque<Message>,
}

impl Channel {
    /// Creates an empty channel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends a message to the back of the channel.
    pub fn send(&mut self, message: Message) {
        self.pending.push_back(message);
    }

    /// Receives the oldest pending message from the channel.
    pub fn receive(&mut self) -> Option<Message> {
        self.pending.pop_front()
    }

    /// Receives all pending messages in FIFO order.
    pub fn receive_pending(&mut self) -> Vec<Message> {
        self.pending.drain(..).collect()
    }

    /// Returns the number of pending messages.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Returns whether the channel has no pending messages.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Synchronous deterministic IPC message bus.
#[derive(Default)]
pub struct MessageBus {
    processes: BTreeMap<ProcessId, Channel>,
    sent: BTreeMap<MessageId, Message>,
    next_message_id: u64,
}

impl MessageBus {
    /// Creates an empty message bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a process and returns whether it was newly inserted.
    pub fn register_process(&mut self, process_id: impl Into<ProcessId>) -> bool {
        self.processes
            .insert(process_id.into(), Channel::new())
            .is_none()
    }

    /// Unregisters a process and returns its channel when it existed.
    pub fn unregister_process(&mut self, process_id: &ProcessId) -> Option<Channel> {
        self.processes.remove(process_id)
    }

    /// Returns whether a process is registered.
    pub fn is_registered(&self, process_id: &ProcessId) -> bool {
        self.processes.contains_key(process_id)
    }

    /// Sends a message to one registered receiver.
    pub fn send(
        &mut self,
        sender: impl Into<ProcessId>,
        receiver: impl Into<ProcessId>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<MessageId, IpcError> {
        self.deliver(sender.into(), receiver.into(), payload.into(), None)
    }

    /// Receives the oldest pending message for a process.
    pub fn receive(&mut self, process_id: &ProcessId) -> Option<Message> {
        self.processes
            .get_mut(process_id)
            .and_then(Channel::receive)
    }

    /// Receives all pending messages for a process in FIFO order.
    pub fn receive_pending(&mut self, process_id: &ProcessId) -> Vec<Message> {
        self.processes
            .get_mut(process_id)
            .map(Channel::receive_pending)
            .unwrap_or_default()
    }

    /// Broadcasts a message to every registered process except the sender.
    pub fn broadcast(
        &mut self,
        sender: impl Into<ProcessId>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Vec<MessageId>, IpcError> {
        let sender = sender.into();
        self.ensure_sender(&sender)?;

        let payload = payload.into();
        let receivers: Vec<_> = self
            .processes
            .keys()
            .filter(|process_id| *process_id != &sender)
            .cloned()
            .collect();

        let mut ids = Vec::with_capacity(receivers.len());
        for receiver in receivers {
            ids.push(self.deliver(sender.clone(), receiver, payload.clone(), None)?);
        }

        Ok(ids)
    }

    /// Sends a request message and returns its ID.
    pub fn request(
        &mut self,
        sender: impl Into<ProcessId>,
        receiver: impl Into<ProcessId>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<MessageId, IpcError> {
        self.send(sender, receiver, payload)
    }

    /// Sends a reply correlated to a prior request.
    pub fn reply(
        &mut self,
        request_id: MessageId,
        sender: impl Into<ProcessId>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<MessageId, IpcError> {
        let request = self
            .sent
            .get(&request_id)
            .ok_or(IpcError::RequestNotFound(request_id))?;

        self.deliver(
            sender.into(),
            request.sender().clone(),
            payload.into(),
            Some(request_id),
        )
    }

    fn deliver(
        &mut self,
        sender: ProcessId,
        receiver: ProcessId,
        payload: Vec<u8>,
        reply_to: Option<MessageId>,
    ) -> Result<MessageId, IpcError> {
        self.ensure_sender(&sender)?;
        self.ensure_receiver(&receiver)?;

        let id = self.next_message_id();
        let message = Message::new(id, sender, receiver.clone(), payload, reply_to);

        self.processes
            .get_mut(&receiver)
            .expect("receiver was checked before delivery")
            .send(message.clone());
        self.sent.insert(id, message);

        Ok(id)
    }

    fn ensure_sender(&self, sender: &ProcessId) -> Result<(), IpcError> {
        if self.processes.contains_key(sender) {
            Ok(())
        } else {
            Err(IpcError::SenderNotRegistered(sender.clone()))
        }
    }

    fn ensure_receiver(&self, receiver: &ProcessId) -> Result<(), IpcError> {
        if self.processes.contains_key(receiver) {
            Ok(())
        } else {
            Err(IpcError::ReceiverNotRegistered(receiver.clone()))
        }
    }

    fn next_message_id(&mut self) -> MessageId {
        self.next_message_id += 1;
        MessageId::new(self.next_message_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{Channel, IpcError, Message, MessageBus, MessageId, ProcessId};

    fn message(id: u64, receiver: &str) -> Message {
        Message::new(
            MessageId::new(id),
            ProcessId::new("sender"),
            ProcessId::new(receiver),
            Vec::new(),
            None,
        )
    }

    #[test]
    fn channel_receives_messages_in_fifo_order() {
        let mut channel = Channel::new();

        channel.send(message(1, "receiver"));
        channel.send(message(2, "receiver"));
        channel.send(message(3, "receiver"));

        assert_eq!(
            channel.receive().map(|message| message.id()),
            Some(MessageId::new(1))
        );
        assert_eq!(
            channel.receive().map(|message| message.id()),
            Some(MessageId::new(2))
        );
        assert_eq!(
            channel.receive().map(|message| message.id()),
            Some(MessageId::new(3))
        );
        assert!(channel.receive().is_none());
    }

    #[test]
    fn sends_messages_to_registered_processes() {
        let mut bus = MessageBus::new();
        let sender = ProcessId::new("composer");
        let receiver = ProcessId::new("reasoner");
        bus.register_process(sender.clone());
        bus.register_process(receiver.clone());

        let id = bus
            .send(sender.clone(), receiver.clone(), b"compose".to_vec())
            .unwrap();
        let received = bus.receive(&receiver).unwrap();

        assert_eq!(received.id(), id);
        assert_eq!(received.sender(), &sender);
        assert_eq!(received.receiver(), &receiver);
        assert_eq!(received.payload(), b"compose");
    }

    #[test]
    fn rejects_unknown_processes() {
        let mut bus = MessageBus::new();
        bus.register_process("sender");

        let error = bus.send("sender", "missing", Vec::new()).unwrap_err();
        assert_eq!(
            error,
            IpcError::ReceiverNotRegistered(ProcessId::new("missing"))
        );

        let error = bus.send("missing", "sender", Vec::new()).unwrap_err();
        assert_eq!(
            error,
            IpcError::SenderNotRegistered(ProcessId::new("missing"))
        );
    }

    #[test]
    fn receives_pending_messages_in_delivery_order() {
        let mut bus = MessageBus::new();
        let receiver = ProcessId::new("receiver");
        bus.register_process("sender");
        bus.register_process(receiver.clone());

        bus.send("sender", receiver.clone(), b"one".to_vec())
            .unwrap();
        bus.send("sender", receiver.clone(), b"two".to_vec())
            .unwrap();

        let pending = bus.receive_pending(&receiver);

        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].payload(), b"one");
        assert_eq!(pending[1].payload(), b"two");
        assert!(bus.receive(&receiver).is_none());
    }

    #[test]
    fn broadcasts_to_registered_receivers_deterministically() {
        let mut bus = MessageBus::new();
        bus.register_process("sender");
        bus.register_process("beta");
        bus.register_process("alpha");

        let ids = bus.broadcast("sender", b"notice".to_vec()).unwrap();

        assert_eq!(ids, vec![MessageId::new(1), MessageId::new(2)]);
        assert_eq!(
            bus.receive(&ProcessId::new("alpha"))
                .map(|message| message.id()),
            Some(MessageId::new(1))
        );
        assert_eq!(
            bus.receive(&ProcessId::new("beta"))
                .map(|message| message.id()),
            Some(MessageId::new(2))
        );
        assert!(bus.receive(&ProcessId::new("sender")).is_none());
    }

    #[test]
    fn request_and_reply_correlate_messages() {
        let mut bus = MessageBus::new();
        let requester = ProcessId::new("requester");
        let responder = ProcessId::new("responder");
        bus.register_process(requester.clone());
        bus.register_process(responder.clone());

        let request_id = bus
            .request(requester.clone(), responder.clone(), b"status".to_vec())
            .unwrap();
        let request = bus.receive(&responder).unwrap();
        let reply_id = bus
            .reply(request.id(), responder.clone(), b"ok".to_vec())
            .unwrap();
        let reply = bus.receive(&requester).unwrap();

        assert_eq!(request.id(), request_id);
        assert_eq!(reply.id(), reply_id);
        assert_eq!(reply.sender(), &responder);
        assert_eq!(reply.receiver(), &requester);
        assert_eq!(reply.reply_to(), Some(request_id));
        assert_eq!(reply.payload(), b"ok");
    }
}
