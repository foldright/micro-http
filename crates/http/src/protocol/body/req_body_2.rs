use crate::protocol::PayloadItem;
use futures::channel::mpsc::SendError;
use futures::{channel::mpsc, SinkExt, Stream, StreamExt};
use tracing::{error, trace, warn};

pub enum BodyRequestSignal {
    RequestData,
    Enough,
}

pub struct BodySender<'conn, S> {
    payload_stream: &'conn mut S,
    signal_receiver: mpsc::Receiver<BodyRequestSignal>,
    data_sender: mpsc::Sender<PayloadItem>,
    eof: bool,
}

impl<'conn, S> BodySender<'conn, S>
where
    S: Stream<Item = PayloadItem> + Unpin,
{
    pub fn new(
        payload_stream: &'conn mut S,
        signal_channel: mpsc::Receiver<BodyRequestSignal>,
        data_channel: mpsc::Sender<PayloadItem>,
    ) -> Self {
        Self { payload_stream, signal_receiver: signal_channel, data_sender: data_channel, eof: false }
    }

    pub async fn start(&mut self) {
        if self.eof {
            return;
        }

        while let Some(signal) = self.signal_receiver.next().await {
            match signal {
                BodyRequestSignal::RequestData => {
                    let payload_item = self.read_data().await;

                    let finished = payload_item.is_eof();

                    if let Err(e) = self.data_sender.send(payload_item).await {
                        error!("failed to send payload body through channel, {}", e);
                    }

                    if finished {
                        self.eof = true;
                        break;
                    }
                }
                BodyRequestSignal::Enough => {
                    break;
                }
            }
        }
    }

    async fn read_data(&mut self) -> PayloadItem {
        match self.payload_stream.next().await {
            Some(payload_item) => payload_item,
            None => unreachable!("body stream should not receive None"),
        }
    }

    pub async fn skip_data(&mut self) {
        if self.eof {
            return;
        }

        loop {
            match self.payload_stream.next().await {
                Some(payload_item) => {
                    if payload_item.is_eof() {
                        self.eof = true;
                        break;
                    }
                }
                None => unreachable!("body stream should not receive None when skip data"),
            }
        }
    }
}

pub struct BodyReceiver {
    signal_sender: mpsc::Sender<BodyRequestSignal>,
    data_receiver: mpsc::Receiver<PayloadItem>,
}

impl BodyReceiver {
    pub async fn receive_data(&mut self) -> Option<PayloadItem> {
        if let Err(e) = self.signal_sender.send(BodyRequestSignal::RequestData).await {
            error!("failed to send request_more through channel, {}", e);
        }

        match self.data_receiver.next().await {
            Some(payload_item) if !payload_item.is_eof() => Some(payload_item),
            Some(_) => None,
            None => unreachable!("body stream should not receive None when receive data"),
        }
    }
}
