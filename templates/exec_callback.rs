// This file is appended to exec/mod.rs by setup-upstream.sh

/// Execute benchmark with a progress callback.
///
/// This is a variant of `par_execute` that calls the provided callback function
/// on each sample, allowing real-time progress monitoring.
pub async fn par_execute_with_callback<F>(
    name: &str,
    exec_options: &ExecutionOptions,
    sampling: Interval,
    workload: Workload,
    show_progress: bool,
    keep_log: bool,
    hdrh_writer: &mut Option<Box<dyn HistogramWriter>>,
    on_sample: F,
) -> Result<BenchmarkStats>
where
    F: Fn(&crate::stats::Sample) + Send + Sync,
{
    if exec_options.cycle_range.1 <= exec_options.cycle_range.0 {
        return Err(LatteError::Configuration(format!(
            "End cycle {} must not be lower than start cycle {}",
            exec_options.cycle_range.1, exec_options.cycle_range.0
        )));
    }

    let thread_count = exec_options.threads.get();
    let concurrency = exec_options.concurrency;
    let rate = exec_options.rate;
    let rate_sine_amplitude = exec_options.rate_sine_amplitude;
    let rate_sine_frequency = 1.0 / exec_options.rate_sine_period.as_secs_f64();
    let progress = match exec_options.duration {
        Interval::Count(count) => Progress::with_count(name.to_string(), count),
        Interval::Time(duration) => Progress::with_duration(name.to_string(), duration),
        Interval::Unbounded => unreachable!(),
    };
    let progress_opts = status_line::Options {
        initially_visible: show_progress,
        ..Default::default()
    };
    let progress = Arc::new(StatusLine::with_options(progress, progress_opts));
    let deadline = BoundedCycleCounter::new(exec_options.duration, exec_options.cycle_range);
    let mut streams = Vec::with_capacity(thread_count);
    let mut stats = Recorder::start(rate, concurrency, keep_log, hdrh_writer);

    for _ in 0..thread_count {
        let s = spawn_stream(
            concurrency,
            rate.map(|r| r / (thread_count as f64)),
            rate_sine_amplitude,
            rate_sine_frequency,
            sampling,
            workload.clone()?,
            deadline.share(),
            progress.clone(),
        );
        streams.push(s);
    }

    loop {
        let partial_stats = receive_one_of_each(&mut streams).await;
        let partial_stats: Vec<_> = partial_stats.into_iter().try_collect()?;
        if partial_stats.is_empty() {
            break Ok(stats.finish());
        }

        let sample = stats.record(&partial_stats);
        on_sample(&sample);
        if sampling.is_bounded() {
            progress.set_visible(false);
            println!("{sample}");
            progress.set_visible(show_progress);
        }
    }
}
