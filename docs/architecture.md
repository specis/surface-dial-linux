# Surface Dial Linux — Architecture UML

```mermaid
classDiagram
    %% ── Traits ─────────────────────────────────────────────────────────────
    class ControlMode {
        <<trait>>
        +meta() ControlModeMeta
        +on_start(haptics) Result
        +on_end(haptics) Result
        +on_btn_press(haptics) Result
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta i32) Result
    }

    %% ── Control Mode Metadata ───────────────────────────────────────────────
    class ControlModeMeta {
        +name: String
        +icon: String
        +haptics: bool
        +steps: u16
    }

    %% ── Built-in Control Modes ──────────────────────────────────────────────
    class Scroll {
        +meta() ControlModeMeta
        +on_dial(haptics, delta) Result
    }
    class ScrollMT {
        -acc_delta: i32
        +meta() ControlModeMeta
        +on_start(haptics) Result
        +on_end(haptics) Result
        +on_btn_press(haptics) Result
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
    }
    class Zoom {
        +meta() ControlModeMeta
        +on_dial(haptics, delta) Result
    }
    class Volume {
        +meta() ControlModeMeta
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
    }
    class Media {
        +meta() ControlModeMeta
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
    }
    class MediaWithVolume {
        -click_tx: Sender
        -release_tx: Sender
        -worker_handle: JoinHandle
        +meta() ControlModeMeta
        +on_btn_press(haptics) Result
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
    }
    class Paddle {
        -_worker: JoinHandle
        -msg: Sender~Msg~
        +meta() ControlModeMeta
        +on_start(haptics) Result
        +on_end(haptics) Result
        +on_btn_press(haptics) Result
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
        drop()
    }
    class ConfigMode {
        +name: String
        +icon: String
        +haptics: bool
        +steps: u16
        -btn_press: Vec~KeyCode~
        -btn_release: Vec~KeyCode~
        -dial_cw: Vec~KeyCode~
        -dial_ccw: Vec~KeyCode~
        +from_spec(spec) Result~Self~
        +meta() ControlModeMeta
        +on_btn_press(haptics) Result
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
    }
    class DbusMode {
        -tx: SyncSender~DbusEvent~
        +new() DbusMode
        +meta() ControlModeMeta
        +on_btn_press(haptics) Result
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
    }
    class DbusEvent {
        <<enumeration>>
        Rotated(i32)
        Pressed
        Released
    }
    class DialInterface {
        <<zbus interface: com.dialmenu.Daemon>>
        +dial_rotated(ctx, delta i32) signal
        +dial_pressed(ctx) signal
        +dial_released(ctx) signal
    }

    %% ── Controller ──────────────────────────────────────────────────────────
    class DialController {
        -device: DialDevice
        -modes: Vec~Box~ControlMode~~
        -active_mode: ActiveMode
        -new_mode: Arc~Mutex~Option~usize~~~
        -meta_mode: Box~ControlMode~
        +new(device, initial_mode, modes) DialController
        +mode_switcher() ModeSwitcher
        +switch_mode_by_name(name) void
        +run() Result
    }
    class ModeSwitcher {
        -new_mode: Arc~Mutex~Option~usize~~~
        -mode_names: Vec~String~
        +switch_to(name)
    }

    %% ── Focus Watcher ───────────────────────────────────────────────────────
    class FocusWatcher {
        +start(switcher ModeSwitcher)$
    }
    class ShellIntrospectProxy {
        <<zbus proxy: org.gnome.Shell.Introspect>>
        +get_windows() Result~HashMap~
    }
    class ProfilesConfig {
        +profile: Vec~Profile~
    }
    class Profile {
        +name: String
        +match_app_id: Option~String~
        +mode: String
    }
    class ActiveMode {
        <<enumeration>>
        Normal(usize)
        Meta
    }
    class MetaMode {
        -metas: Vec~ControlModeMeta~
        -current_mode: usize
        -new_mode: Arc~Mutex~Option~usize~~~
        -first_release: bool
        -notif: Option~NotificationHandle~
        +on_start(haptics) Result
        +on_btn_release(haptics) Result
        +on_dial(haptics, delta) Result
    }

    %% ── Dial Device ─────────────────────────────────────────────────────────
    class DialDevice {
        -long_press_timeout: Duration
        -haptics: DialHaptics
        -events: Receiver~RawInputEvent~
        -possible_long_press: bool
        +new(timeout) Result~DialDevice~
        +next_event() Result~DialEvent~
        +haptics() DialHaptics
    }
    class DialEvent {
        +time: Duration
        +kind: DialEventKind
        +from_raw_evt(evt) Option~DialEvent~
    }
    class DialEventKind {
        <<enumeration>>
        Connect
        Disconnect
        Ignored
        ButtonPress
        ButtonRelease
        Dial(i32)
        ButtonLongPress
    }

    %% ── Events Worker ───────────────────────────────────────────────────────
    class EventsWorker {
        -events: Sender~RawInputEvent~
        -haptics_msg: Sender~DialHapticsWorkerMsg~
        -input_kind: DialInputKind
        +run() io::Result
        -event_loop(device) io::Result
        -udev_to_evdev(device) io::Result~Option~Device~~
    }
    class RawInputEvent {
        <<enumeration>>
        Event(InputEvent)
        Connect
        Disconnect
    }
    class DialInputKind {
        <<enumeration>>
        Control
        MultiAxis
    }

    %% ── Haptics ─────────────────────────────────────────────────────────────
    class DialHaptics {
        -msg: Sender~DialHapticsWorkerMsg~
        +new(msg) Result~DialHaptics~
        +set_mode(haptics, steps) Result
        +buzz(repeat) Result
    }
    class DialHapticsWorker {
        -msg: Receiver~DialHapticsWorkerMsg~
        +new(msg) Result~DialHapticsWorker~
        +run() Result
    }
    class DialHapticsWorkerMsg {
        <<enumeration>>
        DialConnected
        DialDisconnected
        SetMode(haptics, steps)
        Manual(repeat)
    }
    class DialHidWrapper {
        -hid_device: HidDevice
        +set_mode(haptics, steps) Result
        +buzz(repeat) Result
    }

    %% ── Fake Input ──────────────────────────────────────────────────────────
    class FakeInputs {
        <<singleton>>
        +keyboard: Mutex~VirtualDevice~
        +touchpad: Mutex~VirtualDevice~
    }
    class fake_input {
        <<module>>
        +parse_key_code(name) Option~KeyCode~
        +key_click(keys) io::Result
        +key_press(keys) io::Result
        +key_release(keys) io::Result
        +scroll_step(dir) io::Result
        +scroll_mt_start() io::Result
        +scroll_mt_step(delta) io::Result
        +scroll_mt_end() io::Result
    }
    class ScrollStep {
        <<enumeration>>
        Up
        Down
    }

    %% ── Config ──────────────────────────────────────────────────────────────
    class Config {
        +last_mode: usize
        +from_disk() Result~Config~
        +to_disk() Result
    }

    %% ── Error ───────────────────────────────────────────────────────────────
    class Error {
        <<enumeration>>
        ConfigFile(String)
        OpenDevInputDir(io::Error)
        OpenEventFile(PathBuf, io::Error)
        HidError(hidapi::HidError)
        MissingDial
        MultipleDials
        UnexpectedEvt(InputEvent)
        Evdev(io::Error)
        Notif(notify_rust::error::Error)
        TermSig
    }

    %% ── Paddle Worker ───────────────────────────────────────────────────────
    class PaddleWorker {
        -msg: Receiver~Msg~
        -velocity: i32
        -last_delta: i32
        -enabled: bool
        +run()
    }
    class PaddleMsg {
        <<enumeration>>
        Kill
        ButtonDown
        ButtonUp
        Delta(i32)
        Enabled(bool)
    }

    %% ── Relationships ───────────────────────────────────────────────────────

    %% ControlMode implementations
    ControlMode <|.. Scroll
    ControlMode <|.. ScrollMT
    ControlMode <|.. Zoom
    ControlMode <|.. Volume
    ControlMode <|.. Media
    ControlMode <|.. MediaWithVolume
    ControlMode <|.. Paddle
    ControlMode <|.. ConfigMode
    ControlMode <|.. MetaMode
    ControlMode <|.. DbusMode

    %% ControlMode returns metadata
    ControlMode ..> ControlModeMeta : returns

    %% Controller owns modes and device
    DialController *-- DialDevice : owns
    DialController o-- ControlMode : modes[]
    DialController *-- MetaMode : owns
    DialController *-- ActiveMode : active_mode
    DialController --> Config : reads/writes
    DialController ..> ModeSwitcher : produces via mode_switcher()

    %% MetaMode shares new_mode Arc with DialController
    MetaMode ..> DialController : signals via Arc~Mutex~

    %% ModeSwitcher shares new_mode Arc
    ModeSwitcher ..> DialController : writes new_mode via Arc~Mutex~

    %% DbusMode internal structure
    DbusMode *-- DbusEvent : sends via mpsc
    DbusMode ..> DialInterface : emits signals via tokio thread

    %% FocusWatcher
    FocusWatcher ..> ModeSwitcher : calls switch_to()
    FocusWatcher ..> ShellIntrospectProxy : polls get_windows()
    FocusWatcher *-- ProfilesConfig : loads from profiles.toml
    ProfilesConfig *-- Profile

    %% DialDevice owns haptics proxy and receives events
    DialDevice *-- DialHaptics : owns
    DialDevice ..> RawInputEvent : receives via mpsc
    DialDevice ..> DialEvent : produces
    DialEvent *-- DialEventKind

    %% EventsWorker sends to DialDevice and DialHapticsWorker
    EventsWorker ..> RawInputEvent : sends via mpsc
    EventsWorker ..> DialHapticsWorkerMsg : sends via mpsc
    EventsWorker *-- DialInputKind

    %% Haptics chain
    DialHaptics ..> DialHapticsWorkerMsg : sends via mpsc
    DialHapticsWorker ..> DialHapticsWorkerMsg : receives
    DialHapticsWorker *-- DialHidWrapper : creates when connected

    %% Paddle internal worker
    Paddle *-- PaddleWorker : spawns thread
    Paddle ..> PaddleMsg : sends via mpsc
    PaddleWorker ..> PaddleMsg : receives

    %% Fake input usage
    fake_input *-- FakeInputs : lazy_static
    fake_input *-- ScrollStep
    Scroll ..> fake_input : uses
    ScrollMT ..> fake_input : uses
    Zoom ..> fake_input : uses
    Volume ..> fake_input : uses
    Media ..> fake_input : uses
    MediaWithVolume ..> fake_input : uses
    Paddle ..> fake_input : uses
    ConfigMode ..> fake_input : uses
```

## Threading Model

```mermaid
graph TD
    main["main()"] --> sig["Signal Handler Thread\n(polls SIGTERM/SIGINT)"]
    main --> ctrl["controller_main() Thread"]

    ctrl --> fw["FocusWatcher Thread\n(tokio runtime + 1s poll)"]
    ctrl --> dc["DialController::run()"]
    dc --> dd["DialDevice::next_event()"]

    dd -->|"spawns"| ew["EventsWorker Thread\n(udev enum + evdev read)"]
    dd -->|"spawns"| hw["DialHapticsWorker Thread\n(HID device 0x045e:091b)"]

    ew -->|"RawInputEvent (mpsc)"| dd
    ew -->|"DialHapticsWorkerMsg (mpsc)"| hw

    dc -->|"spawns"| pw["PaddleWorker Thread\n(velocity decay loop)"]
    dc -->|"spawns"| mw["MediaWithVolume Worker Thread\n(double-click detection)"]
    dc -->|"spawns (temp)"| sw["ScrollMT startup Thread\n(500ms workaround)"]
    dc -->|"spawns (DbusMode::new)"| db["DbusMode Thread\n(tokio runtime + D-Bus signals)"]

    fw -->|"switch_to() via ModeSwitcher\nArc&lt;Mutex&gt;"| dc
    fw -->|"get_windows() D-Bus call"| gs["org.gnome.Shell.Introspect"]
    db -->|"DialRotated / DialPressed / DialReleased\n(com.dialmenu.Daemon)"| dbus["Session Bus"]
```

## Event Flow: Dial Rotation

```mermaid
sequenceDiagram
    participant HW as Surface Dial HW
    participant EW as EventsWorker
    participant DD as DialDevice
    participant DC as DialController
    participant CM as ControlMode
    participant FI as fake_input / VirtualDevice

    HW->>EW: REL_DIAL evdev event
    EW->>DD: RawInputEvent::Event (mpsc)
    DD->>DD: from_raw_evt → DialEvent::Dial(delta)
    DD->>DC: DialEvent::Dial(delta)
    DC->>CM: on_dial(haptics, delta)
    CM->>FI: key_click / scroll_step / scroll_mt_step
    FI->>FI: VirtualDevice.emit(InputEvent[])
    Note over FI: Kernel forwards to focused app
```

## Event Flow: D-Bus Signal Emission (DbusMode)

```mermaid
sequenceDiagram
    participant DC as DialController
    participant DM as DbusMode
    participant CH as mpsc channel
    participant BG as DbusMode BG Thread (tokio)
    participant BUS as Session Bus

    DC->>DM: on_dial(haptics, delta)
    DM->>CH: try_send(DbusEvent::Rotated(delta))
    BG->>CH: recv()
    BG->>BG: DialInterface::dial_rotated(&signal_ctx, delta).await
    BG->>BUS: signal com.dialmenu.Daemon.DialRotated(delta)
    Note over BUS: Broadcast to all subscribers
```

## Event Flow: Focus-based Mode Switch (FocusWatcher)

```mermaid
sequenceDiagram
    participant FW as FocusWatcher Thread (tokio)
    participant BUS as Session Bus (org.gnome.Shell.Introspect)
    participant MS as ModeSwitcher
    participant DC as DialController (next iteration)

    loop every 1 second
        FW->>BUS: GetWindows()
        BUS-->>FW: {window_id: {app-id, has-focus, ...}}
        FW->>FW: find has-focus=true → app-id
        FW->>FW: match against profiles.toml
        FW->>MS: switch_to("DbusMode")
        MS->>MS: *new_mode.lock() = Some(idx)
    end
    DC->>DC: top of run() loop — picks up new_mode
    DC->>DC: activate new mode
```
