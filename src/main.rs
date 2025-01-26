use std::{
	collections::VecDeque,
	sync::atomic::{AtomicU32, Ordering},
};

/// Loads the test processes.
fn load_test_processes() -> Vec<Process> {
	let list = [
		Process::new(
			[27, 31, 43, 18, 22, 26, 24].into(),
			[5, 3, 5, 4, 6, 4, 3, 4].into(),
		),
		Process::new(
			[48, 44, 42, 37, 76, 41, 31, 43].into(),
			[4, 5, 7, 12, 9, 4, 9, 7, 8].into(),
		),
		Process::new(
			[33, 41, 65, 21, 61, 18, 26, 31].into(),
			[8, 12, 18, 14, 4, 15, 14, 5, 6].into(),
		),
		Process::new(
			[35, 41, 45, 51, 61, 54, 82, 77].into(),
			[3, 4, 5, 3, 4, 5, 6, 5, 3].into(),
		),
		Process::new(
			[24, 21, 36, 26, 31, 28, 21, 13, 11].into(),
			[16, 17, 5, 16, 7, 13, 11, 6, 3, 4].into(),
		),
		Process::new(
			[22, 8, 10, 12, 14, 18, 24, 30].into(),
			[11, 4, 5, 6, 7, 9, 12, 15, 8].into(),
		),
		Process::new(
			[46, 41, 42, 21, 32, 19, 33].into(),
			[14, 17, 11, 15, 4, 7, 16, 10].into(),
		),
		Process::new([14, 33, 51, 73, 87].into(), [4, 5, 6, 14, 16, 6].into()),
	];

	list.into()
}

/// Process ID counter.
static PROCESS_LAST: AtomicU32 = AtomicU32::new(1);

struct Process {
	id: u32,
	next_arrival: u32,
	io_times: VecDeque<u32>,
	cpu_times: VecDeque<u32>,

	turnaround_time: u32,
	waiting_time: u32,
	response_time: u32,
}

impl Process {
	fn new(io_times: VecDeque<u32>, cpu_times: VecDeque<u32>) -> Self {
		// Assign a unique ID to each process.
		let id = PROCESS_LAST.fetch_add(1, Ordering::SeqCst);

		Self {
			id,
			next_arrival: 0,
			io_times,
			cpu_times,

			turnaround_time: 0,
			waiting_time: 0,
			response_time: u32::MAX,
		}
	}
}

/// The response of the scheduler after a step.
#[derive(Default)]
enum Response {
	Success(Process),
	Failure(Process),

	#[default]
	Empty,
}

/// The data returned by the scheduler after a step.
struct Data {
	cpu_time: u32,
	idle_time: u32,

	response: Response,
}

struct RoundRobin {
	processes: VecDeque<Process>,
	quanta: u32,
}

impl RoundRobin {
	// Creates a new scheduler from a list of processes.
	fn from_processes(processes: VecDeque<Process>, quanta: u32) -> Self {
		Self { processes, quanta }
	}

	fn is_empty(&self) -> bool {
		self.processes.is_empty()
	}

	fn io_busy(&self, current_time: u32) -> bool {
		self.processes
			.iter()
			.all(|process| process.next_arrival > current_time)
	}

	// Returns a list of processes that are waiting for IO.
	fn io_remaining(&self, current_time: u32) -> Vec<(u32, u32)> {
		self.processes
			.iter()
			.filter(|&process| (process.next_arrival > current_time))
			.map(|process| (process.id, process.next_arrival - current_time))
			.collect()
	}

	// Returns a list of processes that are waiting for CPU.
	fn cpu_remaining(&self, current_time: u32) -> Vec<(u32, u32)> {
		self.processes
			.iter()
			.filter(|&process| (process.next_arrival <= current_time))
			.map(|process| (process.id, process.cpu_times.front().copied().unwrap()))
			.collect()
	}

	// Steps the scheduler forward by one time unit.
	fn step(&mut self, current_time: u32) -> Data {
		// Get the next process to run.
		let mut process = self.processes.pop_front().unwrap();

		// Calculate the idle time and waiting time.
		let (idle_time, waiting_time) = if process.next_arrival >= current_time {
			(process.next_arrival - current_time, 0)
		} else {
			(0, current_time - process.next_arrival)
		};

		println!("Start P{} at {}", process.id, current_time + idle_time);

		// Run the process for the quanta.
		let cpu_time = process.cpu_times.pop_front().unwrap();
		let (cpu_time, io_time, fail) = if cpu_time > self.quanta {
			// If the process has more CPU time than the quanta, run it again.
			process.cpu_times.push_front(cpu_time - self.quanta);

			(self.quanta, 0, true)
		} else {
			// Otherwise, run the process for the remaining CPU time.
			let io_time = process.io_times.pop_front().unwrap_or(0);

			(cpu_time, io_time, false)
		};

		process.next_arrival = cpu_time + io_time + idle_time + current_time;
		process.waiting_time += waiting_time;
		process.turnaround_time += cpu_time + io_time + waiting_time;
		process.response_time = process.response_time.min(current_time + idle_time);

		// Add the process back to the queue if it still has CPU time remaining.
		let response = if process.cpu_times.is_empty() {
			Response::Success(process)
		} else if fail {
			Response::Failure(process)
		} else {
			self.processes.push_back(process);

			Response::Empty
		};

		Data {
			cpu_time,
			idle_time,
			response,
		}
	}
}

struct FirstComeFirstServe {
	processes: Vec<Process>,
}

impl FirstComeFirstServe {
	// Creates a new scheduler from a list of processes.
	fn from_processes(processes: Vec<Process>) -> Self {
		Self { processes }
	}

	fn is_empty(&self) -> bool {
		self.processes.is_empty()
	}

	// Returns a list of processes that are waiting for IO.
	fn io_remaining(&self, current_time: u32) -> Vec<(u32, u32)> {
		self.processes
			.iter()
			.filter(|&process| (process.next_arrival > current_time))
			.map(|process| (process.id, process.next_arrival - current_time))
			.collect()
	}

	// Returns a list of processes that are waiting for CPU.
	fn cpu_remaining(&self, current_time: u32) -> Vec<(u32, u32)> {
		self.processes
			.iter()
			.filter(|&process| (process.next_arrival <= current_time))
			.map(|process| (process.id, process.cpu_times.front().copied().unwrap()))
			.collect()
	}

	// Find the process with the lowest next arrival time.
	fn find_next_process(&self) -> usize {
		let mut chosen_index = 0;

		for (index, process) in self.processes.iter().enumerate() {
			if process.next_arrival < self.processes[chosen_index].next_arrival {
				chosen_index = index;
			}
		}

		chosen_index
	}

	// Steps the scheduler forward by one time unit.
	fn step(&mut self, current_time: u32) -> Data {
		let process_index = self.find_next_process();
		let process = &mut self.processes[process_index];

		// If the process has not arrived yet, wait until it does.
		let (idle_time, waiting_time) = if process.next_arrival >= current_time {
			(process.next_arrival - current_time, 0)
		} else {
			(0, current_time - process.next_arrival)
		};

		println!("Start P{} at {}", process.id, current_time + idle_time);

		// Pop the next CPU time from the process.
		let cpu_time = process.cpu_times.pop_front().unwrap();
		// Pop the next IO time from the process.
		let io_time = process.io_times.pop_front().unwrap_or(0);

		// Update the process's metrics.
		process.next_arrival = cpu_time + io_time + idle_time + current_time;
		process.waiting_time += waiting_time;
		process.turnaround_time += cpu_time + io_time + waiting_time;
		process.response_time = process.response_time.min(current_time + idle_time);

		// If the process has no more CPU times, remove it from the list.
		let response = if process.cpu_times.is_empty() {
			let process = self.processes.remove(process_index);

			Response::Success(process)
		} else {
			Response::Empty
		};

		Data {
			cpu_time,
			idle_time,
			response,
		}
	}
}

struct MultiLevelFeedbackQueue {
	level_1: RoundRobin,
	level_2: RoundRobin,
	level_3: FirstComeFirstServe,
}

impl MultiLevelFeedbackQueue {
	// Creates a new scheduler from a list of processes.
	fn from_processes(processes: VecDeque<Process>) -> Self {
		Self {
			level_1: RoundRobin::from_processes(processes, 5),
			level_2: RoundRobin::from_processes(VecDeque::new(), 10),
			level_3: FirstComeFirstServe::from_processes(Vec::new()),
		}
	}

	// Prints the list of processes waiting for IO and CPU.
	fn show_lists(&self, current_time: u32) {
		// Chains the lists together and sorts them by the next arrival time for IO and CPU.
		let mut io_list: Vec<_> = self
			.level_1
			.io_remaining(current_time)
			.into_iter()
			.chain(self.level_2.io_remaining(current_time))
			.chain(self.level_3.io_remaining(current_time))
			.collect();

		io_list.sort_unstable_by_key(|data| data.0);

		let mut cpu_list: Vec<_> = self
			.level_1
			.cpu_remaining(current_time)
			.into_iter()
			.chain(self.level_2.cpu_remaining(current_time))
			.chain(self.level_3.cpu_remaining(current_time))
			.collect();

		cpu_list.sort_unstable_by_key(|data| data.0);

		// Print the IO list if it is not empty.
		if !io_list.is_empty() {
			print!("IO: ");

			for (id, time) in io_list {
				print!("(P{id} {time}) ");
			}

			println!();
		}

		// Print the CPU list if it is not empty.
		if !cpu_list.is_empty() {
			print!("CPU: ");

			for (id, time) in cpu_list {
				print!("(P{id} {time}) ");
			}

			println!();
		}
	}

	fn is_empty(&self) -> bool {
		self.level_1.is_empty() && self.level_2.is_empty() && self.level_3.is_empty()
	}

	// Steps the scheduler forward by one time unit.
	fn step(&mut self, current_time: u32) -> Data {
		self.show_lists(current_time);

		// If the first level is not empty and not busy, run it.
		if !self.level_1.is_empty() && !self.level_1.io_busy(current_time) {
			let mut data = self.level_1.step(current_time);

			// If the process failed, downgrade it to the second level.
			if let Response::Failure(process) = data.response {
				println!("Downgraded P{} to level 2", process.id);

				self.level_2.processes.push_back(process);

				data.response = Response::Empty;
			}

			data
		// If the second level is not empty and not busy, run it.
		} else if !self.level_2.is_empty() && !self.level_2.io_busy(current_time) {
			let mut data = self.level_2.step(current_time);

			// If the process failed, downgrade it to the third level.
			if let Response::Failure(process) = data.response {
				println!("Downgraded P{} to level 3", process.id);

				self.level_3.processes.push(process);

				data.response = Response::Empty;
			}

			data
		// Otherwise, run the third level.
		} else {
			self.level_3.step(current_time)
		}
	}
}

fn main() {
	let processes = load_test_processes();
	let process_count = processes.len() as f64;

	let mut scheduler = MultiLevelFeedbackQueue::from_processes(processes.into());

	let mut total_turnaround_time = 0;
	let mut total_waiting_time = 0;
	let mut total_response_time = 0;
	let mut idle_time = 0;
	let mut current_time = 0;

	// Handle the response from the scheduler.
	while !scheduler.is_empty() {
		let data = scheduler.step(current_time);

		match data.response {
			Response::Success(process) => {
				total_turnaround_time += process.turnaround_time;
				total_waiting_time += process.waiting_time;
				total_response_time += process.response_time;

				println!(
					"End P{} with Turnaround Time: {}, Waiting Time: {}, Response Time: {}",
					process.id,
					process.turnaround_time,
					process.waiting_time,
					process.response_time
				);
			}
			Response::Failure(process) => panic!("P{} failed", process.id),
			Response::Empty => {}
		}

		idle_time += data.idle_time;
		current_time += data.cpu_time + data.idle_time;

		println!();
	}

	let turnaround_average = f64::from(total_turnaround_time) / process_count;
	let waiting_average = f64::from(total_waiting_time) / process_count;
	let response_average = f64::from(total_response_time) / process_count;
	let cpu_utilization = (1.0 - f64::from(idle_time) / f64::from(current_time)) * 100.0;

	println!("Turnaround Time: {turnaround_average:.2}");
	println!("Waiting Time: {waiting_average:.2}");
	println!("Response Time: {response_average:.2}");
	println!("CPU Utilization: {cpu_utilization:.2}%");
}
