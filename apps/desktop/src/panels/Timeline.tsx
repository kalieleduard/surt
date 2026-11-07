import {
  TIMELINE,
  TIMELINE_LANES,
  TIMELINE_MINUTES,
  TIMELINE_NOW,
  TRANSIENT_PAUSE,
} from "../data/activity";

const TICK_MINUTES = 10;
const START_HOUR = 9;

function percent(minute: number) {
  return `${(minute / TIMELINE_MINUTES) * 100}%`;
}

function clock(minute: number) {
  const hour = START_HOUR + Math.floor(minute / 60);
  return `${String(hour).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
}

export function Timeline() {
  const ticks = Array.from({ length: Math.floor(TIMELINE_MINUTES / TICK_MINUTES) + 1 }, (_, index) => index * TICK_MINUTES);
  return (
    <section className="timeline">
      <header className="timeline__head">
        <span className="caption">Executions</span>
        <span className="timeline__legend">
          <i className="state-dot state-running" /> running
          <i className="state-dot state-succeeded" /> succeeded
          <i className="state-dot state-blocked" /> failed attempt
          <i className="state-dot state-waiting" /> waiting on you
        </span>
      </header>
      <div className="timeline__grid">
        <div className="timeline__lanes">
          {TIMELINE_LANES.map((lane) => (
            <span key={lane} className="timeline__lane-name caption">{lane}</span>
          ))}
        </div>
        <div className="timeline__track">
          <div className="timeline__ruler">
            {ticks.map((tick) => (
              <span key={tick} style={{ left: percent(tick) }} className="mono">{clock(tick)}</span>
            ))}
          </div>
          <div
            className="timeline__pause"
            style={{ left: percent(TRANSIENT_PAUSE.start), width: percent(TRANSIENT_PAUSE.end - TRANSIENT_PAUSE.start) }}
            title="Transient Failure: agent launches paused"
          >
            <span className="mono">{TRANSIENT_PAUSE.label}</span>
          </div>
          {TIMELINE_LANES.map((lane, row) => (
            <div key={lane} className="timeline__row" style={{ top: 26 + row * 34 }}>
              {TIMELINE.filter((bar) => bar.lane === lane).map((bar) => (
                <span
                  key={bar.label + bar.start}
                  className={`timeline__bar state-${bar.state}`}
                  style={{ left: percent(bar.start), width: percent(bar.end - bar.start) }}
                  title={bar.label}
                >
                  {bar.label}
                </span>
              ))}
            </div>
          ))}
          <div className="timeline__now" style={{ left: percent(TIMELINE_NOW) }}>
            <span className="mono">{clock(TIMELINE_NOW)}</span>
          </div>
        </div>
      </div>
    </section>
  );
}
