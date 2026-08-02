use std::io::{self, Write};

use n2o5::progress::{Progress, ProgressConfig, ProgressStatus};
use n2o5::{BuildGraph, BuildId};

use crate::output;

pub struct ConsoleProgress;

impl Progress for ConsoleProgress {
    fn prepare(&self, _config: &ProgressConfig) {}

    fn build_started(&self, graph: &BuildGraph, id: BuildId, status: &ProgressStatus) {
        let build = graph.lookup_build(id).expect("invalid build id");
        let counter = format!("[{}/{}]", status.started + 1, status.total);
        println!(
            "{} {}",
            output::accent_stdout(&counter, output::CYAN),
            build.human_readable()
        );
    }

    fn stdout_line(&self, _graph: &BuildGraph, _id: BuildId, chunk: &[u8]) {
        io::stdout().write_all(chunk).unwrap();
    }

    fn build_finished(
        &self,
        graph: &BuildGraph,
        id: BuildId,
        success: bool,
        _status: &ProgressStatus,
    ) {
        if success {
            return;
        }

        let build = graph.lookup_build(id).expect("invalid build id");
        eprintln!(
            "{} {}",
            output::tag_stderr("fail", output::RED),
            build.human_readable()
        );
    }

    fn finish(&self) {}
}
