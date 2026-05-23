#import <Cocoa/Cocoa.h>
#import <CoreFoundation/CoreFoundation.h>
#import <OpenGL/gl.h>
#import <dispatch/dispatch.h>
#import <mpv/render.h>
#import <mpv/render_gl.h>

@interface OpenPlayerMpvGLView : NSOpenGLView
@property(nonatomic, assign) mpv_render_context *mpvContext;
@property(atomic, assign) BOOL renderScheduled;
@end

@implementation OpenPlayerMpvGLView

- (instancetype)initWithFrame:(NSRect)frame {
    NSOpenGLPixelFormatAttribute attributes[] = {
        NSOpenGLPFAAccelerated,
        NSOpenGLPFADoubleBuffer,
        NSOpenGLPFAColorSize,
        24,
        NSOpenGLPFAAlphaSize,
        8,
        0,
    };
    NSOpenGLPixelFormat *format =
        [[NSOpenGLPixelFormat alloc] initWithAttributes:attributes];
    self = [super initWithFrame:frame pixelFormat:format];
    if (self) {
        self.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
        self.wantsBestResolutionOpenGLSurface = YES;
        self.mpvContext = NULL;
        self.renderScheduled = NO;

        [[self openGLContext] makeCurrentContext];
        GLint swapInterval = 1;
        [[self openGLContext] setValues:&swapInterval
                            forParameter:NSOpenGLCPSwapInterval];
    }
    return self;
}

- (void)prepareOpenGL {
    [super prepareOpenGL];
    [[self openGLContext] makeCurrentContext];
    glClearColor(0.0, 0.0, 0.0, 1.0);
}

- (void)reshape {
    [super reshape];
    [self setNeedsDisplay:YES];
}

- (void)drawRect:(NSRect)dirtyRect {
    (void)dirtyRect;
    [[self openGLContext] makeCurrentContext];

    NSRect backingBounds = [self convertRectToBacking:self.bounds];
    int width = (int)backingBounds.size.width;
    int height = (int)backingBounds.size.height;
    if (width < 1) {
        width = 1;
    }
    if (height < 1) {
        height = 1;
    }

    glViewport(0, 0, width, height);
    if (self.mpvContext) {
        mpv_opengl_fbo fbo = {
            .fbo = 0,
            .w = width,
            .h = height,
            .internal_format = 0,
        };
        int flipY = 1;
        mpv_render_param params[] = {
            {MPV_RENDER_PARAM_OPENGL_FBO, &fbo},
            {MPV_RENDER_PARAM_FLIP_Y, &flipY},
            {0, NULL},
        };
        mpv_render_context_render(self.mpvContext, params);
        mpv_render_context_report_swap(self.mpvContext);
    } else {
        glClear(GL_COLOR_BUFFER_BIT);
    }

    [[self openGLContext] flushBuffer];
}

@end

static void run_on_main_sync(dispatch_block_t block) {
    if ([NSThread isMainThread]) {
        block();
    } else {
        dispatch_sync(dispatch_get_main_queue(), block);
    }
}

void *openplayer_mpv_gl_view_create(void *parent_ptr) {
    if (!parent_ptr) {
        return NULL;
    }

    __block OpenPlayerMpvGLView *created = nil;
    run_on_main_sync(^{
      NSView *parent = (__bridge NSView *)parent_ptr;
      OpenPlayerMpvGLView *view =
          [[OpenPlayerMpvGLView alloc] initWithFrame:parent.bounds];
      [parent addSubview:view];
      created = view;
    });

    return created ? (void *)CFBridgingRetain(created) : NULL;
}

void openplayer_mpv_gl_view_remove(void *view_ptr) {
    if (!view_ptr) {
        return;
    }

    run_on_main_sync(^{
      OpenPlayerMpvGLView *view = CFBridgingRelease((CFTypeRef)view_ptr);
      view.mpvContext = NULL;
      [view removeFromSuperview];
      [view clearGLContext];
    });
}

void openplayer_mpv_gl_view_resize(void *view_ptr) {
    if (!view_ptr) {
        return;
    }

    run_on_main_sync(^{
      OpenPlayerMpvGLView *view = (__bridge OpenPlayerMpvGLView *)view_ptr;
      if (view.superview) {
          view.frame = view.superview.bounds;
      }
      [view setNeedsDisplay:YES];
    });
}

void openplayer_mpv_gl_view_set_render_context(void *view_ptr, void *render_context) {
    if (!view_ptr) {
        return;
    }

    run_on_main_sync(^{
      OpenPlayerMpvGLView *view = (__bridge OpenPlayerMpvGLView *)view_ptr;
      view.mpvContext = (mpv_render_context *)render_context;
      [view setNeedsDisplay:YES];
    });
}

void openplayer_mpv_gl_view_make_current(void *view_ptr) {
    if (!view_ptr) {
        return;
    }

    run_on_main_sync(^{
      OpenPlayerMpvGLView *view = (__bridge OpenPlayerMpvGLView *)view_ptr;
      [[view openGLContext] makeCurrentContext];
    });
}

void openplayer_mpv_gl_view_draw(void *view_ptr) {
    if (!view_ptr) {
        return;
    }

    OpenPlayerMpvGLView *view = (__bridge OpenPlayerMpvGLView *)view_ptr;
    if (view.renderScheduled) {
        return;
    }

    view.renderScheduled = YES;
    id retainedView = CFBridgingRelease(CFRetain((__bridge CFTypeRef)view));
    dispatch_async(dispatch_get_main_queue(), ^{
      OpenPlayerMpvGLView *strongView = retainedView;
      strongView.renderScheduled = NO;
      if (strongView.window) {
          [strongView setNeedsDisplay:YES];
      }
    });
}

void *openplayer_mpv_gl_get_proc_address(const char *name) {
    if (!name) {
        return NULL;
    }

    CFStringRef symbolName =
        CFStringCreateWithCString(kCFAllocatorDefault, name, kCFStringEncodingASCII);
    if (!symbolName) {
        return NULL;
    }

    CFBundleRef bundle =
        CFBundleGetBundleWithIdentifier(CFSTR("com.apple.opengl"));
    void *address = bundle ? CFBundleGetFunctionPointerForName(bundle, symbolName) : NULL;
    CFRelease(symbolName);
    return address;
}

void openplayer_macos_prepare_main_window(void *main_view_ptr) {
    if (!main_view_ptr) {
        return;
    }

    run_on_main_sync(^{
      NSView *mainView = (__bridge NSView *)main_view_ptr;
      NSWindow *mainWindow = mainView.window;
      if (!mainWindow) {
          return;
      }

      mainWindow.titleVisibility = NSWindowTitleHidden;
      mainWindow.titlebarAppearsTransparent = YES;
      mainWindow.styleMask = mainWindow.styleMask | NSWindowStyleMaskFullSizeContentView;

      NSArray<NSNumber *> *buttonTypes = @[
          @(NSWindowCloseButton),
          @(NSWindowMiniaturizeButton),
          @(NSWindowZoomButton),
      ];
      for (NSNumber *buttonType in buttonTypes) {
          NSButton *button = [mainWindow standardWindowButton:buttonType.integerValue];
          button.hidden = YES;
          button.enabled = NO;
      }
    });
}

void openplayer_macos_prepare_overlay_window(void *main_view_ptr, void *overlay_view_ptr) {
    if (!main_view_ptr || !overlay_view_ptr) {
        return;
    }

    run_on_main_sync(^{
      NSView *mainView = (__bridge NSView *)main_view_ptr;
      NSView *overlayView = (__bridge NSView *)overlay_view_ptr;
      NSWindow *mainWindow = mainView.window;
      NSWindow *overlayWindow = overlayView.window;
      if (!mainWindow || !overlayWindow) {
          return;
      }

      overlayWindow.collectionBehavior =
          overlayWindow.collectionBehavior | NSWindowCollectionBehaviorFullScreenAuxiliary;
      if (overlayWindow.parentWindow != mainWindow) {
          [overlayWindow.parentWindow removeChildWindow:overlayWindow];
          [mainWindow addChildWindow:overlayWindow ordered:NSWindowAbove];
      }
    });
}
