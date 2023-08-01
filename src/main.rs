#![allow(non_snake_case, non_upper_case_globals)]

use std::{
    ffi::CStr,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicI32},
};

use icrate::{
    ns_string,
    AppKit::{
        NSApplication, NSApplicationActivationPolicyRegular, NSApplicationDelegate,
        NSApplicationTerminateReply, NSBackingStoreBuffered, NSClosableWindowMask,
        NSDeviceIndependentModifierFlagsMask, NSEventTypeFlagsChanged, NSEventTypeKeyDown,
        NSEventTypeKeyUp, NSEventTypeLeftMouseDown, NSEventTypeLeftMouseDragged,
        NSEventTypeLeftMouseUp, NSEventTypeMouseMoved, NSEventTypeOtherMouseDown,
        NSEventTypeOtherMouseDragged, NSEventTypeOtherMouseUp, NSEventTypeRightMouseDown,
        NSEventTypeRightMouseDragged, NSEventTypeRightMouseUp, NSEventTypeScrollWheel, NSMenu,
        NSMenuItem, NSResizableWindowMask, NSResponder, NSTitledWindowMask, NSWindow,
        NSWindowController, NSWindowDelegate,
    },
    Foundation::{
        CGPoint, CGSize, NSDate, NSDefaultRunLoopMode, NSNotification, NSProcessInfo, NSRect,
    },
};
use objc2::{
    declare_class,
    ffi::NSUIntegerMax,
    msg_send, msg_send_id,
    mutability::InteriorMutable,
    rc::{autoreleasepool, Id},
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    sel, ClassType,
};

bitflags::bitflags! {
    struct NSEventModifierFlags: u8 {
        const ALPHA_SHIFT = 0x01;
        const SHIFT = 0x02;
        const CONTROL = 0x04;
        const ALTERNATE = 0x08;
        const COMMAND = 0x10;
        const NUMERIC_PAD = 0x20;
        const HELP = 0x40;
        const FUNCTION = 0x80;
    }
}

#[link(name = "Cocoa", kind = "framework")]
extern "C" {
    static NSApp: Id<NSApplication>;
}

static TERMINATE: AtomicBool = AtomicBool::new(false);

declare_class!(
    #[derive(Debug)]
    struct AppDelegate;

    unsafe impl ClassType for AppDelegate {
        #[inherits(NSObject)]
        type Super = NSResponder;
        type Mutability = InteriorMutable;
        const NAME: &'static str = "AppDelegate";
    }

    unsafe impl AppDelegate {
        #[method(init)]
        unsafe fn init(this: *mut Self) -> Option<NonNull<Self>> {
            unsafe { msg_send![super(this), init] }
        }
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationShouldTerminate:)]
        unsafe fn applicationShouldTerminate(
            &self,
            _sender: &NSApplication,
        ) -> NSApplicationTerminateReply {
            println!("requested to terminate");
            TERMINATE.store(true, std::sync::atomic::Ordering::SeqCst);
            0
        }
    }
);

impl AppDelegate {
    fn new() -> Id<Self> {
        unsafe { msg_send_id![Self::alloc(), init] }
    }
}

static WINDOW_COUNT: AtomicI32 = AtomicI32::new(0);

declare_class!(
    #[derive(Debug)]
    struct WindowDelegate;

    unsafe impl ClassType for WindowDelegate {
        #[inherits(NSObject)]
        type Super = NSResponder;
        type Mutability = InteriorMutable;
        const NAME: &'static str = "WindowDelegate";
    }

    unsafe impl WindowDelegate {
        #[method(init)]
        unsafe fn init(this: *mut Self) -> Option<NonNull<Self>> {
            unsafe { msg_send![super(this), init] }
        }
    }

    unsafe impl NSObjectProtocol for WindowDelegate {}

    unsafe impl NSWindowDelegate for WindowDelegate {
        #[method(windowWillClose:)]
        unsafe fn window_will_close(&self, _sender: &NSNotification) {
            println!("window will close");
            assert!(WINDOW_COUNT.load(std::sync::atomic::Ordering::SeqCst) > 0);
            TERMINATE.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
);

impl WindowDelegate {
    fn new() -> Id<Self> {
        unsafe { msg_send_id![Self::alloc(), init] }
    }
}

fn main() {
    autoreleasepool(|pool| {
        let _ = unsafe { NSApplication::sharedApplication() };

        unsafe { NSApp.setActivationPolicy(NSApplicationActivationPolicyRegular) };

        let dg = AppDelegate::new();
        unsafe { NSApp.setDelegate(Some(&ProtocolObject::from_id(dg))) };
        unsafe { NSApp.finishLaunching() };

        let menu_bar = Id::autorelease(unsafe { NSMenu::new() }, pool);
        let app_menu_item = Id::autorelease(unsafe { NSMenuItem::new() }, pool);
        unsafe { menu_bar.addItem(&app_menu_item) };
        unsafe { NSApp.setMainMenu(Some(&menu_bar)) };

        let app_menu = Id::autorelease(unsafe { NSMenu::new() }, pool);
        let proc_info = Id::autorelease(NSProcessInfo::processInfo(), pool);
        let app_name = Id::autorelease(proc_info.processName(), pool);
        let quit_title = Id::autorelease(ns_string!("Quit ").stringByAppendingString(&app_name), pool);

        let terminate = sel!(terminate:);
        let key_equivalent = ns_string!("q");
        let quit_menu_item = Id::autorelease(unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(),
                &quit_title,
                Some(terminate),
                key_equivalent,
            )
        }, pool);
        unsafe { app_menu.addItem(&quit_menu_item) };
        unsafe { app_menu_item.setSubmenu(Some(&app_menu)) };

        let rect = NSRect::new(CGPoint::new(0.0, 0.0), CGSize::new(500.0, 500.0));
        let window_style = NSTitledWindowMask | NSClosableWindowMask | NSResizableWindowMask;

        let window = Id::autorelease(
            unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(),
                    rect,
                    window_style,
                    NSBackingStoreBuffered,
                    false,
                )
            },
            pool,
        );
        let _window_controller = Id::autorelease(
            unsafe {
                NSWindowController::initWithWindow(NSWindowController::alloc(), Some(&window))
            },
            pool,
        );

        unsafe { window.setReleasedWhenClosed(false) };

        WINDOW_COUNT.store(1, std::sync::atomic::Ordering::SeqCst);

        let wdg = WindowDelegate::new();
        unsafe { window.setDelegate(Some(&ProtocolObject::from_id(wdg))) };

        let content_view = Id::autorelease(unsafe { window.contentView() }.unwrap(), pool);
        unsafe { content_view.setWantsBestResolutionOpenGLSurface(true) };

        unsafe { window.cascadeTopLeftFromPoint(CGPoint::new(20.0, 20.0)) };
        unsafe { window.setTitle(ns_string!("sup from Rust")) };
        unsafe { window.makeKeyAndOrderFront(Some(&window)) };
        unsafe { window.setAcceptsMouseMovedEvents(true) };

        while !TERMINATE.load(std::sync::atomic::Ordering::SeqCst) {
            let Some(event) = (unsafe {
                NSApp.nextEventMatchingMask_untilDate_inMode_dequeue(
                    NSUIntegerMax as _,
                    Some(&NSDate::distantPast()),
                    NSDefaultRunLoopMode,
                    true,
                )
            }) else {
                continue;
            };

            match unsafe { event.r#type() } {
                NSEventTypeMouseMoved
                | NSEventTypeLeftMouseDragged
                | NSEventTypeRightMouseDragged
                | NSEventTypeOtherMouseDragged => {
                    let current_window = unsafe { NSApp.keyWindow() }.unwrap();
                    let current_window_content_view =
                        unsafe { current_window.contentView().unwrap() };
                    let adjust_frame = unsafe { current_window_content_view.frame() };
                    let p = unsafe { current_window.mouseLocationOutsideOfEventStream() };
                    let p = CGPoint::new(
                        p.x.clamp(0.0, adjust_frame.size.width),
                        p.y.clamp(0.0, adjust_frame.size.height),
                    );
                    let r = NSRect::new(p, CGSize::new(0.0, 0.0));
                    let r = unsafe { current_window_content_view.convertRectToBacking(r) };
                    let p = r.origin;

                    println!("mouse moved to {} {}", p.x, p.y);
                }
                NSEventTypeLeftMouseDown => println!("mouse left key down"),
                NSEventTypeLeftMouseUp => println!("mouse left key up"),
                NSEventTypeRightMouseDown => println!("mouse right key down"),
                NSEventTypeRightMouseUp => println!("mouse right key up"),
                NSEventTypeOtherMouseDown => {
                    println!("mouse other key down : {}", unsafe { event.buttonNumber() })
                }
                NSEventTypeOtherMouseUp => {
                    println!("mouse other key up : {}", unsafe { event.buttonNumber() })
                }
                NSEventTypeScrollWheel => {
                    let scroll_factor = if unsafe { event.hasPreciseScrollingDeltas() } {
                        0.1
                    } else {
                        1.0
                    };
                    let dx = unsafe { event.scrollingDeltaX() } * scroll_factor;
                    let dy = unsafe { event.scrollingDeltaX() } * scroll_factor;
                    if dx.abs() > 0.0 || dy.abs() > 0.0 {
                        println!("mouse scroll wheel delta {} {}", dx, dy);
                    }
                }
                NSEventTypeFlagsChanged => {
                    let modifiers = unsafe { event.modifierFlags() };
                    let keys = NSEventModifierFlags::from_bits(
                        ((modifiers & NSDeviceIndependentModifierFlagsMask) >> 16) as _,
                    )
                    .unwrap();
                    println!(
                        "mod keys : mask {:03} state {}{}{}{}{}{}{}{}\n",
                        keys.bits(),
                        keys.contains(NSEventModifierFlags::ALPHA_SHIFT),
                        keys.contains(NSEventModifierFlags::SHIFT),
                        keys.contains(NSEventModifierFlags::CONTROL),
                        keys.contains(NSEventModifierFlags::ALTERNATE),
                        keys.contains(NSEventModifierFlags::COMMAND),
                        keys.contains(NSEventModifierFlags::NUMERIC_PAD),
                        keys.contains(NSEventModifierFlags::HELP),
                        keys.contains(NSEventModifierFlags::FUNCTION)
                    )
                }
                NSEventTypeKeyDown => {
                    let input_text_utf8 =
                        unsafe { CStr::from_ptr(event.characters().unwrap().UTF8String()) }
                            .to_str()
                            .unwrap();
                    let key_code = unsafe { event.keyCode() };
                    println!("key down {}, text '{}'", key_code, input_text_utf8);
                }
                NSEventTypeKeyUp => println!("key up {}", unsafe { event.keyCode() }),
                _ => {}
            };

            unsafe { NSApp.sendEvent(&event) };
            if TERMINATE.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            unsafe { NSApp.updateWindows() };
        }
    });
}
