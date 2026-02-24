/// Result type for notification attempts, indicating whether it was sent,
/// skipped, or if there was an error.
pub enum NotificationResult {
    /// Indicates that it's not yet time to send the notification based on the configured intervals.
    NotYetTime,

    /// Indicates that the notification was processed as a dry run, meaning
    /// it was printed to the console instead of being sent.
    DryRun,

    /// Indicates that the notification was successful.
    Success,

    /// Indicates that the notification failed.
    Failure(String),
}
