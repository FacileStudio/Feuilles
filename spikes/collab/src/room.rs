use std::collections::VecDeque;

use crate::{Client, Downstream, Edit, Server, Stats, from_bytes, new_model};

/// One scheduler step of a simulated run.
#[derive(Debug, Clone)]
pub enum Step {
    /// Client `n % clients` makes a local edit and queues its PUSH.
    Edit(usize, Edit),
    /// The server takes the oldest PUSH from client `n % clients`.
    Deliver(usize),
    /// Client `n % clients` takes the oldest frame the server sent it.
    Receive(usize),
}

/// The server, the clients and the in-flight frames between them.
pub struct Room {
    /// The server.
    pub server: Server,
    /// The clients.
    pub clients: Vec<Client>,
    /// Per client, PUSH frames on their way to the server.
    pub upstream: Vec<VecDeque<(u32, Vec<u8>)>>,
    /// Per client, frames on their way from the server.
    pub downstream: Vec<VecDeque<Downstream>>,
    /// Counters.
    pub stats: Stats,
}

impl Room {
    /// Creates a room with `clients` clients, all joined at `seq` 0.
    pub fn new(clients: usize) -> Self {
        let snapshot = new_model().to_bytes();
        Room {
            server: Server::new(&snapshot),
            clients: (1..=clients as u32)
                .map(|id| Client::new(id, &snapshot))
                .collect(),
            upstream: vec![VecDeque::new(); clients],
            downstream: vec![VecDeque::new(); clients],
            stats: Stats::default(),
        }
    }

    /// Runs one step. Returns an error when an invariant breaks.
    pub fn step(&mut self, step: &Step) -> Result<(), String> {
        let n = self.clients.len();
        match step {
            Step::Edit(c, edit) => {
                let c = c % n;
                if let Some(push) = self.clients[c].local_edit(edit, &mut self.stats) {
                    self.upstream[c].push_back(push);
                }
            }
            Step::Deliver(c) => self.deliver(c % n),
            Step::Receive(c) => {
                let c = c % n;
                if let Some(frame) = self.downstream[c].pop_front() {
                    self.clients[c].receive(frame, &mut self.stats);
                }
            }
        }
        self.check_idle_views()
    }

    fn deliver(&mut self, c: usize) {
        let Some((client_seq, payload)) = self.upstream[c].pop_front() else {
            return;
        };
        let id = self.clients[c].id;
        match self.server.push(id, client_seq, payload, &mut self.stats) {
            Ok(update) => {
                for queue in &mut self.downstream {
                    queue.push_back(Downstream::Update(update.clone()));
                }
            }
            Err(_) => self.downstream[c].push_back(Downstream::Reject { client_seq }),
        }
    }

    /// Delivers and receives every frame in flight.
    pub fn drain(&mut self) -> Result<(), String> {
        while self.upstream.iter().any(|q| !q.is_empty())
            || self.downstream.iter().any(|q| !q.is_empty())
        {
            for c in 0..self.clients.len() {
                self.step(&Step::Deliver(c))?;
                self.step(&Step::Receive(c))?;
            }
        }
        Ok(())
    }

    /// Checks that every client with no pending batch has `view` equal to `confirmed`.
    pub fn check_idle_views(&self) -> Result<(), String> {
        let diverged = self.clients.iter().find(|client| {
            client.pending.is_empty() && client.view.to_bytes() != client.confirmed.to_bytes()
        });
        match diverged {
            Some(client) => Err(format!(
                "client {} has no pending batch but view differs from confirmed",
                client.id
            )),
            None => Ok(()),
        }
    }

    /// Checks that the server and every confirmed model have equal bytes.
    pub fn check_converged(&self) -> Result<(), String> {
        let server = self.server.model.to_bytes();
        for client in &self.clients {
            if client.confirmed.to_bytes() != server {
                return Err(format!(
                    "client {} confirmed differs from server",
                    client.id
                ));
            }
            if client.last_seq != self.server.seq {
                return Err(format!("client {} missed updates", client.id));
            }
        }
        if from_bytes(&server).to_bytes() != server {
            return Err("server snapshot does not round-trip".to_string());
        }
        Ok(())
    }
}

/// Runs a full case: every step, then a drain, then the convergence checks.
pub fn run(clients: usize, steps: &[Step]) -> Result<Stats, String> {
    let mut room = Room::new(clients);
    for step in steps {
        room.step(step)?;
    }
    room.drain()?;
    room.check_converged()?;
    room.check_idle_views()?;
    Ok(room.stats)
}
