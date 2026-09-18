use std::{
    cell::Cell,
    num::NonZeroIsize,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{COLOR_BTNFACE, DEFAULT_GUI_FONT, GetStockObject, GetSysColorBrush},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::*, HiDpi::GetDpiForSystem, Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::*,
        },
    },
    core::{HSTRING, w},
};

const RUNNING: u8 = 0;
const COMPLETED: u8 = 1;
const CANCELLED: u8 = 2;

// Luaと音声更新が同時に開いている間は、最後の処理が終わるまで操作を禁止する。
static OWNER_LOCKS: Mutex<Vec<(NonZeroIsize, usize, bool)>> = Mutex::new(Vec::new());

struct OwnerLock(NonZeroIsize);

impl OwnerLock {
    fn new(owner: NonZeroIsize) -> Self {
        let mut owners = OWNER_LOCKS.lock().unwrap();
        if let Some((_, count, _)) = owners.iter_mut().find(|(hwnd, _, _)| *hwnd == owner) {
            *count += 1;
        } else {
            let was_disabled =
                unsafe { EnableWindow(HWND(owner.get() as *mut _), false) }.as_bool();
            owners.push((owner, 1, !was_disabled));
        }
        Self(owner)
    }
}

impl Drop for OwnerLock {
    fn drop(&mut self) {
        let mut owners = OWNER_LOCKS.lock().unwrap();
        let index = owners
            .iter()
            .position(|(hwnd, _, _)| *hwnd == self.0)
            .unwrap();
        owners[index].1 -= 1;
        if owners[index].1 == 0 {
            let (owner, _, restore) = owners.swap_remove(index);
            if restore {
                let _ = unsafe { EnableWindow(HWND(owner.get() as *mut _), true) };
            }
        }
    }
}

#[derive(Default)]
pub struct ProgressState {
    status: AtomicU8,
    position: AtomicU32,
}

impl ProgressState {
    pub fn running(&self) -> bool {
        self.status.load(Ordering::Acquire) == RUNNING
    }

    pub fn update(&self, percent: f64) -> bool {
        if !percent.is_finite() || !self.running() {
            return false;
        }
        self.position.store(
            (percent.clamp(0.0, 100.0) * 100.0).round() as u32,
            Ordering::Relaxed,
        );
        if percent >= 100.0 {
            self.complete()
        } else {
            self.running()
        }
    }

    // 完了と×操作は先に成功した方を採用する。音声の確定直前にも使用する。
    pub fn complete(&self) -> bool {
        self.status
            .compare_exchange(RUNNING, COMPLETED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn cancel(&self) -> bool {
        self.status
            .compare_exchange(RUNNING, CANCELLED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

pub struct Progress {
    pub state: Arc<ProgressState>,
    worker: Option<JoinHandle<()>>,
    owner_lock: Option<OwnerLock>,
}

impl Progress {
    pub fn new(title: String, color: u32, owner: NonZeroIsize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !title.contains('\0') && color <= 0xffffff,
            "Invalid progress window parameters"
        );
        let owner_lock = OwnerLock::new(owner);
        let state = Arc::new(ProgressState::default());
        let shared = Arc::clone(&state);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("rikky-progress".into())
            .spawn(move || {
                if let Err(error) =
                    run_window(title, color, HWND(owner.get() as *mut _), &shared, &sender)
                {
                    tracing::error!("進捗ウィンドウの処理に失敗しました: {error:#}");
                    let _ = sender.send(Err(error));
                }
                shared.cancel();
            })?;
        let progress = Self {
            state,
            worker: Some(worker),
            owner_lock: Some(owner_lock),
        };
        receiver.recv()??;
        Ok(progress)
    }

    #[cfg(test)]
    pub fn headless() -> Self {
        Self {
            state: Arc::new(ProgressState::default()),
            worker: None,
            owner_lock: None,
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.state.cancel();
        // ホストのWndProcやメッセージループには手を加えない。
        // 進捗スレッドを待つ前に、呼び出し側で入力の制限を解除する。
        drop(self.owner_lock.take());
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            tracing::error!("進捗ウィンドウのスレッドが異常終了しました");
        }
    }
}

// 旧版のLua APIは全スクリプトで1ウィンドウ。音声更新は独立したProgressを所有する。
static LUA_PROGRESS: Mutex<Option<Progress>> = Mutex::new(None);

pub fn start(title: String, color: u32, owner: NonZeroIsize) -> bool {
    if title.contains('\0') || color > 0xffffff {
        return false;
    }
    let mut window = LUA_PROGRESS.lock().unwrap();
    drop(window.take());
    match Progress::new(title, color, owner) {
        Ok(progress) => {
            *window = Some(progress);
            true
        }
        Err(error) => {
            tracing::error!("進捗ウィンドウを開けません: {error:#}");
            false
        }
    }
}

pub fn processing(percent: f64) -> bool {
    let mut window = LUA_PROGRESS.lock().unwrap();
    let Some(progress) = window.as_ref() else {
        return false;
    };
    let success = progress.state.update(percent);
    if !progress.state.running() {
        drop(window.take());
    }
    success
}

pub fn end() -> bool {
    let window = LUA_PROGRESS.lock().unwrap().take();
    window
        .as_ref()
        .is_some_and(|progress| progress.state.cancel())
}

struct WindowClass {
    name: HSTRING,
    instance: HINSTANCE,
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        // 登録したスレッドで、ウィンドウを破棄した後に解除する。
        if let Err(error) = unsafe { UnregisterClassW(&self.name, Some(self.instance)) } {
            tracing::error!("進捗ウィンドウクラスを解除できません: {error}");
        }
    }
}

struct Window<'a>(&'a WindowUi);

impl Drop for Window<'_> {
    fn drop(&mut self) {
        let hwnd = self.0.hwnd.get();
        if !hwnd.is_invalid() {
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

struct WindowUi {
    state: Arc<ProgressState>,
    hwnd: Cell<HWND>,
    bar: Cell<HWND>,
    label: Cell<HWND>,
    displayed: Cell<u32>,
}

fn run_window(
    title: String,
    color: u32,
    owner: HWND,
    state: &Arc<ProgressState>,
    ready: &mpsc::SyncSender<anyhow::Result<()>>,
) -> anyhow::Result<()> {
    static NEXT_CLASS: AtomicU64 = AtomicU64::new(0);
    // HWNDとWin32の操作はこのスレッドだけに閉じ込める。ホストの編集APIは呼ばない。
    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        })
        .ok()?;
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        let name = HSTRING::from(format!(
            "rikky_modoki.progress.{}",
            NEXT_CLASS.fetch_add(1, Ordering::Relaxed)
        ));
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: windows::core::PCWSTR(name.as_ptr()),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..Default::default()
        };
        anyhow::ensure!(
            RegisterClassW(&class) != 0,
            "RegisterClassW failed: {}",
            windows::core::Error::from_thread()
        );
        let class = WindowClass { name, instance };
        let ui = Box::new(WindowUi {
            state: Arc::clone(state),
            hwnd: Cell::default(),
            bar: Cell::default(),
            label: Cell::default(),
            displayed: Cell::new(u32::MAX),
        });
        let scale = |value: i32| value * GetDpiForSystem() as i32 / 96;
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_DLGMODALFRAME,
            &class.name,
            &HSTRING::from(title),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            scale(340),
            scale(110),
            Some(owner),
            None,
            Some(instance),
            Some((&*ui as *const WindowUi).cast()),
        )?;
        let _window = Window(&ui);
        let bar = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PROGRESS_CLASSW,
            w!(""),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(PBS_SMOOTH),
            scale(12),
            scale(12),
            scale(300),
            scale(20),
            Some(hwnd),
            None,
            Some(instance),
            None,
        )?;
        ui.bar.set(bar);
        SetWindowTheme(bar, w!(""), w!(""))?;
        SendMessageW(bar, PBM_SETRANGE32, Some(WPARAM(0)), Some(LPARAM(10_000)));
        let colorref = (color & 0xff) << 16 | color & 0xff00 | color >> 16;
        SendMessageW(bar, PBM_SETBARCOLOR, None, Some(LPARAM(colorref as isize)));
        let label = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("0.0%"),
            WS_CHILD | WS_VISIBLE,
            scale(12),
            scale(38),
            scale(300),
            scale(20),
            Some(hwnd),
            None,
            Some(instance),
            None,
        )?;
        ui.label.set(label);
        SendMessageW(
            label,
            WM_SETFONT,
            Some(WPARAM(GetStockObject(DEFAULT_GUI_FONT).0 as usize)),
            Some(LPARAM(1)),
        );
        anyhow::ensure!(SetTimer(Some(hwnd), 1, 33, None) != 0, "SetTimer failed");
        // 描画や編集処理中の親ウィンドウへ、同期的なアクティブ化を要求しない。
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        ready.send(Ok(()))?;
        let mut message = MSG::default();
        loop {
            match GetMessageW(&mut message, None, 0, 0).0 {
                -1 => return Err(windows::core::Error::from_thread().into()),
                0 => break,
                _ => {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
    }
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // CreateWindowExWからDestroyWindowまでWindowUiのBoxが生存している。
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            (*(create.lpCreateParams as *const WindowUi)).hwnd.set(hwnd);
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowUi;
        if !pointer.is_null() {
            let ui = &*pointer;
            match message {
                WM_CLOSE => {
                    ui.state.cancel();
                    let _ = DestroyWindow(hwnd);
                    return LRESULT(0);
                }
                WM_KEYDOWN if wparam.0 == 0x1b => {
                    ui.state.cancel();
                    let _ = DestroyWindow(hwnd);
                    return LRESULT(0);
                }
                WM_TIMER => {
                    if !ui.state.running() {
                        let _ = DestroyWindow(hwnd);
                    } else {
                        let position = ui.state.position.load(Ordering::Relaxed);
                        if ui.displayed.replace(position) != position {
                            SendMessageW(
                                ui.bar.get(),
                                PBM_SETPOS,
                                Some(WPARAM(position as usize)),
                                None,
                            );
                            let _ = SetWindowTextW(
                                ui.label.get(),
                                &HSTRING::from(format!("{:.1}%", f64::from(position) / 100.0)),
                            );
                        }
                    }
                    return LRESULT(0);
                }
                WM_DESTROY => {
                    ui.state.cancel();
                    PostQuitMessage(0);
                    return LRESULT(0);
                }
                WM_NCDESTROY => {
                    ui.hwnd.set(HWND::default());
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}
