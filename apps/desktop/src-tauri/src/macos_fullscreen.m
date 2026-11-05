#import <Cocoa/Cocoa.h>
#import <dispatch/dispatch.h>
#import <objc/runtime.h>

static char frameKey;
static char standardFrameKey;
static char observerKey;
typedef void (*OpenPlayerFullscreenCallback)(void *, bool);

@interface OpenPlayerWindowRestoreObserver : NSObject
@property(nonatomic, weak) NSWindow *window;
@property(nonatomic, weak) NSWindow *overlay;
@property(nonatomic, assign) void *context;
@property(nonatomic, assign) OpenPlayerFullscreenCallback callback;
- (void)finish:(BOOL)restored;
- (void)restoreFrame:(NSValue *)frame;
@end

@implementation OpenPlayerWindowRestoreObserver

- (void)finish:(BOOL)restored {
    if (!self.context) return;
    __attribute__((objc_precise_lifetime)) OpenPlayerWindowRestoreObserver *keepAlive = self;
    void *context = self.context;
    OpenPlayerFullscreenCallback callback = self.callback;
    self.context = NULL;
    [[NSNotificationCenter defaultCenter] removeObserver:self];
    NSWindow *window = self.window;
    if (window && objc_getAssociatedObject(window, &observerKey) == self) {
        objc_setAssociatedObject(window, &observerKey, nil, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    }
    callback(context, restored);
    (void)keepAlive;
}

- (void)windowClosed:(NSNotification *)notification {
    (void)notification;
    [self finish:NO];
}

- (void)restoreFrame:(NSValue *)frame {
    NSWindow *window = self.window;
    if (!self.context || !window || !frame) {
        [self finish:NO];
        return;
    }
    [window setFrame:frame.rectValue display:YES];
    if (self.overlay) [self.overlay setFrame:frame.rectValue display:YES];
    [self finish:YES];
}

- (void)didExitFullscreen:(NSNotification *)notification {
    (void)notification;
    [[NSNotificationCenter defaultCenter] removeObserver:self
                                                    name:NSWindowDidExitFullScreenNotification
                                                  object:self.window];
    // Tao also dispatches its style and maximize restoration from this notification.
    dispatch_async(dispatch_get_main_queue(), ^{
      dispatch_async(dispatch_get_main_queue(), ^{
        NSWindow *window = self.window;
        NSValue *frame = window ? objc_getAssociatedObject(window, &frameKey) : nil;
        [self restoreFrame:frame];
      });
    });
}
@end

bool openplayer_macos_capture_standard_frame(void *window_ptr) {
    NSWindow *window = (__bridge NSWindow *)window_ptr;
    if (!window || ![NSThread isMainThread]) return false;
    objc_setAssociatedObject(window, &standardFrameKey, [NSValue valueWithRect:window.frame],
                             OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    return true;
}

bool openplayer_macos_has_standard_frame(void *window_ptr) {
    NSWindow *window = (__bridge NSWindow *)window_ptr;
    return window && [NSThread isMainThread] && objc_getAssociatedObject(window, &standardFrameKey);
}

bool openplayer_macos_capture_fullscreen_frame(void *window_ptr) {
    NSWindow *window = (__bridge NSWindow *)window_ptr;
    if (!window || ![NSThread isMainThread]) return false;
    objc_setAssociatedObject(window, &frameKey, [NSValue valueWithRect:window.frame],
                             OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    return true;
}

bool openplayer_macos_observe_fullscreen_exit(void *window_ptr, void *overlay_ptr,
                                              OpenPlayerFullscreenCallback callback, void *context) {
    NSWindow *window = (__bridge NSWindow *)window_ptr;
    if (!window || ![NSThread isMainThread] || !callback || !context ||
        !objc_getAssociatedObject(window, &frameKey) || objc_getAssociatedObject(window, &observerKey)) {
        return false;
    }
    OpenPlayerWindowRestoreObserver *observer = [OpenPlayerWindowRestoreObserver new];
    observer.window = window;
    observer.overlay = (__bridge NSWindow *)overlay_ptr;
    observer.callback = callback;
    observer.context = context;
    objc_setAssociatedObject(window, &observerKey, observer, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    NSNotificationCenter *center = [NSNotificationCenter defaultCenter];
    [center addObserver:observer selector:@selector(didExitFullscreen:)
                   name:NSWindowDidExitFullScreenNotification object:window];
    [center addObserver:observer selector:@selector(windowClosed:)
                   name:NSWindowWillCloseNotification object:window];
    return true;
}

void openplayer_macos_cancel_fullscreen_exit(void *window_ptr) {
    NSWindow *window = (__bridge NSWindow *)window_ptr;
    OpenPlayerWindowRestoreObserver *observer = window ? objc_getAssociatedObject(window, &observerKey) : nil;
    [observer finish:NO];
}

bool openplayer_macos_restore_standard_frame(void *window_ptr, void *overlay_ptr,
                                             OpenPlayerFullscreenCallback callback, void *context) {
    NSWindow *window = (__bridge NSWindow *)window_ptr;
    NSValue *frame = window ? objc_getAssociatedObject(window, &standardFrameKey) : nil;
    if (!window || ![NSThread isMainThread] || !frame || !callback || !context ||
        objc_getAssociatedObject(window, &observerKey)) {
        return false;
    }
    OpenPlayerWindowRestoreObserver *observer = [OpenPlayerWindowRestoreObserver new];
    observer.window = window;
    observer.overlay = (__bridge NSWindow *)overlay_ptr;
    observer.callback = callback;
    observer.context = context;
    objc_setAssociatedObject(window, &observerKey, observer, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    [[NSNotificationCenter defaultCenter] addObserver:observer selector:@selector(windowClosed:)
                                                 name:NSWindowWillCloseNotification object:window];
    // Tao queues its own unmaximize operation before this block.
    dispatch_async(dispatch_get_main_queue(), ^{
      dispatch_async(dispatch_get_main_queue(), ^{
        [observer restoreFrame:frame];
      });
    });
    return true;
}

void openplayer_macos_align_window_frames(void *target_ptr, void *source_ptr) {
    if (!target_ptr || !source_ptr) return;
    dispatch_block_t align = ^{
      NSWindow *target = ((__bridge NSView *)target_ptr).window;
      NSWindow *source = ((__bridge NSView *)source_ptr).window;
      if (target && source && !NSEqualRects(target.frame, source.frame)) {
          [target setFrame:source.frame display:YES];
      }
    };
    if ([NSThread isMainThread]) align();
    else dispatch_sync(dispatch_get_main_queue(), align);
}
