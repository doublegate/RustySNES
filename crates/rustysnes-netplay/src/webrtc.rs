//! The browser WebRTC [`Transport`], `wasm32` only — wraps an already-open
//! [`web_sys::RtcDataChannel`].
//!
//! SDP offer/answer negotiation (`RtcPeerConnection`, async by nature) is deliberately NOT here:
//! it's connection-establishment/signaling glue, which is frontend-owned in this project
//! (matching `wasm_audio.rs`'s own crate-boundary precedent — this crate stays pure protocol/
//! session logic, the frontend owns orchestration). The frontend constructs the
//! `RtcPeerConnection` + `RtcDataChannel`, waits for the channel's `open` event, then hands the
//! open channel to [`WebRtcTransport::new`].

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, RtcDataChannel, RtcDataChannelType};

use crate::message::NetMessage;
use crate::transport::Transport;

/// A [`Transport`] over an already-open WebRTC data channel.
pub struct WebRtcTransport {
    channel: RtcDataChannel,
    incoming: Rc<RefCell<VecDeque<NetMessage>>>,
    // Kept alive for the transport's lifetime — dropping it would detach the JS callback and
    // silently stop delivering messages (the classic wasm-bindgen `Closure` footgun).
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl WebRtcTransport {
    /// Wrap `channel` (assumed already open) as a [`Transport`]. Installs the message handler
    /// that decodes each incoming binary frame as a [`NetMessage`], silently dropping anything
    /// that isn't a binary `ArrayBuffer` or doesn't decode (untrusted input from the wire).
    #[must_use]
    pub fn new(channel: RtcDataChannel) -> Self {
        channel.set_binary_type(RtcDataChannelType::Arraybuffer);
        let incoming: Rc<RefCell<VecDeque<NetMessage>>> = Rc::new(RefCell::new(VecDeque::new()));
        let incoming_for_closure = Rc::clone(&incoming);
        let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
            let Ok(buf) = event.data().dyn_into::<js_sys::ArrayBuffer>() else {
                return;
            };
            let bytes = js_sys::Uint8Array::new(&buf).to_vec();
            if let Ok(msg) = NetMessage::decode(&bytes) {
                incoming_for_closure.borrow_mut().push_back(msg);
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        Self {
            channel,
            incoming,
            _on_message: on_message,
        }
    }

    /// The wrapped channel's current `readyState` (`"connecting"`, `"open"`, `"closing"`, or
    /// `"closed"`) — the frontend uses this to detect a dropped peer.
    #[must_use]
    pub fn ready_state(&self) -> web_sys::RtcDataChannelState {
        self.channel.ready_state()
    }
}

impl Transport for WebRtcTransport {
    fn send(&mut self, msg: &NetMessage) {
        // Best-effort by design, same as the native UDP transport — a send failure here (the
        // channel closed mid-flight, the browser's send buffer is full) is not a protocol
        // violation, just a dropped message like any other.
        let _ = self.channel.send_with_u8_array(&msg.encode());
    }

    fn poll(&mut self) -> Vec<NetMessage> {
        self.incoming.borrow_mut().drain(..).collect()
    }
}
