use ipc::{
    BrowserState, OutputEvent, OutputEventKind, OutputQueueState, ProcessState, StatusCounters,
    StatusSnapshot,
};

#[test]
fn status_snapshot_reports_updated_counters() {
    let mut counters = StatusCounters::default();
    counters.record_frame_submitted();
    counters.record_frame_published();
    counters.record_frame_published();
    counters.record_paint_dropped();
    counters.record_output_dropped();

    let snapshot = StatusSnapshot {
        process_state: ProcessState::Running,
        browser_state: BrowserState::Ready,
        counters,
        last_error: Some("late frame".to_string()),
    };

    assert_eq!(snapshot.counters.submitted_frame_count, 1);
    assert_eq!(snapshot.counters.published_frame_count, 2);
    assert_eq!(snapshot.counters.dropped_paint_count, 1);
    assert_eq!(snapshot.counters.dropped_output_count, 1);

    assert_eq!(
        snapshot.to_lines(),
        vec![
            "process_state=running".to_string(),
            "browser_state=ready".to_string(),
            "submitted_frame_count=1".to_string(),
            "published_frame_count=2".to_string(),
            "dropped_paint_count=1".to_string(),
            "dropped_output_count=1".to_string(),
            "last_error=late frame".to_string(),
        ]
    );
    assert_eq!(
        snapshot.encode_json_like_string(),
        "{\"process_state\":\"running\",\"browser_state\":\"ready\",\"submitted_frame_count\":1,\"published_frame_count\":2,\"dropped_paint_count\":1,\"dropped_output_count\":1,\"last_error\":\"late frame\"}"
    );
    assert_eq!(ProcessState::Starting.as_str(), "starting");
    assert_eq!(ProcessState::Stopping.as_str(), "stopping");
    assert_eq!(ProcessState::Stopped.as_str(), "stopped");
    assert_eq!(ProcessState::Failed.as_str(), "failed");
    assert_eq!(BrowserState::Creating.as_str(), "creating");
    assert_eq!(BrowserState::Loading.as_str(), "loading");
    assert_eq!(BrowserState::Closing.as_str(), "closing");
    assert_eq!(BrowserState::Closed.as_str(), "closed");
    assert_eq!(BrowserState::Failed.as_str(), "failed");
}

#[test]
fn output_queue_preserves_fifo_order() {
    let mut output = OutputQueueState::new(2);
    assert!(output.is_empty());
    assert_eq!(output.capacity(), 2);

    output
        .push_event(OutputEvent::new(OutputEventKind::Console, 1, "first"))
        .unwrap();
    output
        .push_event(OutputEvent::new(
            OutputEventKind::JavaScriptResult,
            2,
            "second",
        ))
        .unwrap();

    assert_eq!(
        output.pop_event().unwrap(),
        Some(OutputEvent::new(OutputEventKind::Console, 1, "first"))
    );
    assert_eq!(
        output.pop_event().unwrap(),
        Some(OutputEvent::new(
            OutputEventKind::JavaScriptResult,
            2,
            "second"
        ))
    );
    assert!(matches!(
        OutputEvent::new(OutputEventKind::PageError, 3, "page").kind,
        OutputEventKind::PageError
    ));
    assert!(matches!(
        OutputEvent::new(OutputEventKind::Navigation, 4, "nav").kind,
        OutputEventKind::Navigation
    ));
    assert_eq!(output.pop_event().unwrap(), None);
}

#[test]
fn output_queue_reports_overflow_and_dropped_count() {
    let mut output = OutputQueueState::new(1);
    output
        .push_event(OutputEvent::new(OutputEventKind::Console, 1, "first"))
        .unwrap();

    let error = output
        .push_event(OutputEvent::new(OutputEventKind::Console, 2, "overflow"))
        .unwrap_err();

    assert!(error.to_string().contains("output event queue is full"));
    assert_eq!(output.dropped_count(), 1);
    assert_eq!(output.len(), 1);
    assert_eq!(
        output.pop_event().unwrap(),
        Some(OutputEvent::new(OutputEventKind::Console, 1, "first"))
    );
}
