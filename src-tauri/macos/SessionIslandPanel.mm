#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>
#import "SessionIslandBridge.h"

static const CGFloat kCompactMinWidth = 154.0;
static const CGFloat kCompactMaxWidth = 220.0;
static const CGFloat kVirtualCompactWidth = 168.0;
static const CGFloat kCompactHeight = 34.0;
static const CGFloat kExpandedWidth = 480.0;
static const CGFloat kExpandedHeight = 156.0;
static const CGFloat kToastWidth = 320.0;
static const CGFloat kToastHeight = 44.0;

static SmalltalkIslandActionCallback gActionCallback = NULL;

typedef struct {
    NSRect screenFrame;
    NSRect visibleFrame;
    NSEdgeInsets safeInsets;
    NSRect auxiliaryTopLeft;
    NSRect auxiliaryTopRight;
    bool hasTopCameraHousing;
    NSRect inferredNotchRect;
} SmalltalkNotchGeometry;

static long long SmalltalkNowMillis(void) {
    return (long long)floor([[NSDate date] timeIntervalSince1970] * 1000.0);
}

static NSString *SmalltalkStringOrEmpty(id value) {
    if ([value isKindOfClass:NSString.class]) {
        return (NSString *)value;
    }
    return @"";
}

static NSString *SmalltalkCleanLabel(NSString *value) {
    if (value.length == 0) {
        return @"";
    }
    NSArray<NSString *> *parts = [value componentsSeparatedByCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
    NSMutableArray<NSString *> *kept = [NSMutableArray array];
    for (NSString *part in parts) {
        if (part.length > 0) {
            [kept addObject:part];
        }
    }
    return [kept componentsJoinedByString:@" "];
}

static NSString *SmalltalkFormatElapsed(long long elapsedMs) {
    long long totalSeconds = MAX(0, elapsedMs / 1000);
    long long hours = totalSeconds / 3600;
    long long minutes = (totalSeconds / 60) % 60;
    long long seconds = totalSeconds % 60;
    if (hours > 0) {
        return [NSString stringWithFormat:@"%lld:%02lld:%02lld", hours, minutes, seconds];
    }
    return [NSString stringWithFormat:@"%02lld:%02lld", minutes, seconds];
}

static NSString *SmalltalkHumanizeTrigger(NSString *value) {
    if (value.length == 0) {
        return @"Waiting for first capture";
    }
    NSString *clean = [[value stringByReplacingOccurrencesOfString:@"_" withString:@" "] lowercaseString];
    if (clean.length == 0) {
        return @"Waiting for first capture";
    }
    return [clean stringByReplacingCharactersInRange:NSMakeRange(0, 1)
                                          withString:[[clean substringToIndex:1] uppercaseString]];
}

static CGFloat SmalltalkClamp(CGFloat value, CGFloat minimum, CGFloat maximum) {
    return MIN(MAX(value, minimum), maximum);
}

static NSTextField *SmalltalkLabel(NSFont *font, CGFloat alpha) {
    NSTextField *label = [NSTextField labelWithString:@""];
    label.font = font;
    label.textColor = [NSColor colorWithWhite:1.0 alpha:alpha];
    label.backgroundColor = NSColor.clearColor;
    label.drawsBackground = NO;
    label.bezeled = NO;
    label.editable = NO;
    label.selectable = NO;
    label.maximumNumberOfLines = 1;
    label.cell.lineBreakMode = NSLineBreakByTruncatingTail;
    return label;
}

static void SmalltalkSendAction(NSString *action) {
    SmalltalkIslandActionCallback callback = gActionCallback;
    if (!callback || action.length == 0) {
        return;
    }
    NSString *json = [NSString stringWithFormat:@"{\"action\":\"%@\"}", action];
    callback(json.UTF8String);
}

static SmalltalkNotchGeometry SmalltalkComputeNotchGeometry(NSScreen *screen) {
    SmalltalkNotchGeometry g;
    g.screenFrame = screen.frame;
    g.visibleFrame = screen.visibleFrame;
    g.safeInsets = NSEdgeInsetsMake(0, 0, 0, 0);
    g.auxiliaryTopLeft = NSZeroRect;
    g.auxiliaryTopRight = NSZeroRect;
    g.hasTopCameraHousing = false;
    g.inferredNotchRect = NSZeroRect;

    if ([screen respondsToSelector:@selector(safeAreaInsets)]) {
        g.safeInsets = screen.safeAreaInsets;
    }
    if ([screen respondsToSelector:@selector(auxiliaryTopLeftArea)]) {
        g.auxiliaryTopLeft = screen.auxiliaryTopLeftArea;
    }
    if ([screen respondsToSelector:@selector(auxiliaryTopRightArea)]) {
        g.auxiliaryTopRight = screen.auxiliaryTopRightArea;
    }

    g.hasTopCameraHousing = g.safeInsets.top > 0.5
        && !NSIsEmptyRect(g.auxiliaryTopLeft)
        && !NSIsEmptyRect(g.auxiliaryTopRight);

    if (g.hasTopCameraHousing) {
        CGFloat notchMinX = NSMaxX(g.auxiliaryTopLeft);
        CGFloat notchMaxX = NSMinX(g.auxiliaryTopRight);
        CGFloat notchHeight = g.safeInsets.top;
        CGFloat notchWidth = MAX(0.0, notchMaxX - notchMinX);
        if (notchWidth > 20.0) {
            g.inferredNotchRect = NSMakeRect(
                notchMinX,
                NSMaxY(g.screenFrame) - notchHeight,
                notchWidth,
                notchHeight
            );
        } else {
            g.hasTopCameraHousing = false;
        }
    }

    return g;
}

static NSScreen *SmalltalkTargetScreen(void) {
    NSPoint mouse = NSEvent.mouseLocation;
    for (NSScreen *screen in NSScreen.screens) {
        if (NSMouseInRect(mouse, screen.frame, NO)) {
            return screen;
        }
    }
    if (NSScreen.mainScreen) {
        return NSScreen.mainScreen;
    }
    return NSScreen.screens.firstObject;
}

static NSSize SmalltalkCompactSizeForScreen(NSScreen *screen) {
    SmalltalkNotchGeometry g = SmalltalkComputeNotchGeometry(screen);
    if (g.hasTopCameraHousing && !NSIsEmptyRect(g.inferredNotchRect)) {
        CGFloat width = SmalltalkClamp(NSWidth(g.inferredNotchRect) + 24.0, kCompactMinWidth, kCompactMaxWidth);
        return NSMakeSize(width, kCompactHeight);
    }
    return NSMakeSize(kVirtualCompactWidth, kCompactHeight);
}

static NSRect SmalltalkIslandFrame(NSScreen *screen, NSSize size) {
    SmalltalkNotchGeometry g = SmalltalkComputeNotchGeometry(screen);
    CGFloat centerX = NSMidX(g.screenFrame);
    if (g.hasTopCameraHousing && !NSIsEmptyRect(g.inferredNotchRect)) {
        centerX = NSMidX(g.inferredNotchRect);
    }

    CGFloat y = NSMaxY(g.screenFrame) - size.height;
    return NSMakeRect(centerX - size.width / 2.0, y, size.width, size.height);
}

@interface SmalltalkIslandViewModel : NSObject
@property(nonatomic, copy) NSString *state;
@property(nonatomic, copy) NSString *sessionId;
@property(nonatomic) long long elapsedMs;
@property(nonatomic) long long frameCount;
@property(nonatomic, copy) NSString *currentApp;
@property(nonatomic, copy) NSString *currentWindow;
@property(nonatomic, copy) NSString *currentSurfaceKind;
@property(nonatomic, copy) NSString *lastTrigger;
@property(nonatomic, copy) NSString *lastError;
@property(nonatomic, copy) NSString *privacyLabel;
@property(nonatomic) BOOL isSensitive;
@end

@implementation SmalltalkIslandViewModel
- (instancetype)init {
    self = [super init];
    if (self) {
        _state = @"hidden";
        _sessionId = @"";
        _currentApp = @"";
        _currentWindow = @"";
        _currentSurfaceKind = @"";
        _lastTrigger = @"";
        _lastError = @"";
        _privacyLabel = @"";
    }
    return self;
}
@end

@interface SmalltalkIslandPanel : NSPanel
@end

@implementation SmalltalkIslandPanel
- (BOOL)canBecomeKeyWindow { return NO; }
- (BOOL)canBecomeMainWindow { return NO; }
@end

@interface SmalltalkIslandButton : NSButton
@property(nonatomic, copy) NSString *islandAction;
@end

@implementation SmalltalkIslandButton
- (BOOL)acceptsFirstMouse:(NSEvent *)event { return YES; }
@end

@interface SmalltalkIslandRootView : NSView
- (void)updateWithViewModel:(SmalltalkIslandViewModel *)model expanded:(BOOL)expanded reduceMotion:(BOOL)reduceMotion;
@end

@interface SmalltalkIslandRootView ()
@property(nonatomic, strong) SmalltalkIslandViewModel *model;
@property(nonatomic) BOOL expanded;
@property(nonatomic) BOOL reduceMotion;
@property(nonatomic) long long modelReceivedAtMs;
@property(nonatomic, strong) NSTimer *timer;
@property(nonatomic, strong) NSView *dotView;
@property(nonatomic, strong) NSProgressIndicator *spinner;
@property(nonatomic, strong) NSTextField *statusLabel;
@property(nonatomic, strong) NSTextField *elapsedLabel;
@property(nonatomic, strong) NSTextField *currentLabel;
@property(nonatomic, strong) NSTextField *lastLabel;
@property(nonatomic, strong) NSTextField *factsLabel;
@property(nonatomic, strong) SmalltalkIslandButton *stopButton;
@property(nonatomic, strong) SmalltalkIslandButton *openButton;
@property(nonatomic, strong) SmalltalkIslandButton *resumeButton;
@property(nonatomic, strong) SmalltalkIslandButton *dismissButton;
@property(nonatomic, strong) CAShapeLayer *shapeMaskLayer;
@end

@implementation SmalltalkIslandRootView
- (instancetype)initWithFrame:(NSRect)frameRect {
    self = [super initWithFrame:frameRect];
    if (self) {
        self.wantsLayer = YES;
        self.layer.backgroundColor = [[NSColor colorWithWhite:0.0 alpha:0.96] CGColor];
        self.layer.masksToBounds = NO;
        _shapeMaskLayer = [CAShapeLayer layer];
        self.layer.mask = _shapeMaskLayer;

        _dotView = [[NSView alloc] initWithFrame:NSZeroRect];
        _dotView.wantsLayer = YES;
        _dotView.layer.backgroundColor = NSColor.systemRedColor.CGColor;
        _dotView.layer.cornerRadius = 3.5;
        [self addSubview:_dotView];

        _spinner = [[NSProgressIndicator alloc] initWithFrame:NSZeroRect];
        _spinner.style = NSProgressIndicatorStyleSpinning;
        _spinner.controlSize = NSControlSizeSmall;
        _spinner.displayedWhenStopped = NO;
        [self addSubview:_spinner];

        _statusLabel = SmalltalkLabel([NSFont systemFontOfSize:12 weight:NSFontWeightSemibold], 0.94);
        _elapsedLabel = SmalltalkLabel([NSFont monospacedDigitSystemFontOfSize:11 weight:NSFontWeightMedium], 0.84);
        _currentLabel = SmalltalkLabel([NSFont systemFontOfSize:13 weight:NSFontWeightSemibold], 0.92);
        _lastLabel = SmalltalkLabel([NSFont systemFontOfSize:12 weight:NSFontWeightRegular], 0.68);
        _factsLabel = SmalltalkLabel([NSFont systemFontOfSize:12 weight:NSFontWeightRegular], 0.62);
        [self addSubview:_statusLabel];
        [self addSubview:_elapsedLabel];
        [self addSubview:_currentLabel];
        [self addSubview:_lastLabel];
        [self addSubview:_factsLabel];

        _stopButton = [self buttonWithTitle:@"" action:@"stop_capture" fill:[NSColor colorWithRed:0.78 green:0.14 blue:0.12 alpha:0.82]];
        _openButton = [self buttonWithTitle:@"Open Smalltalk" action:@"open_main_window" fill:[NSColor colorWithWhite:1.0 alpha:0.13]];
        _resumeButton = [self buttonWithTitle:@"Resume me" action:@"resume_me" fill:[NSColor colorWithWhite:1.0 alpha:0.10]];
        _dismissButton = [self buttonWithTitle:@"Dismiss" action:@"collapse" fill:[NSColor colorWithWhite:1.0 alpha:0.10]];
        [self addSubview:_stopButton];
        [self addSubview:_openButton];
        [self addSubview:_resumeButton];
        [self addSubview:_dismissButton];

        _model = [[SmalltalkIslandViewModel alloc] init];
        _modelReceivedAtMs = SmalltalkNowMillis();
        [self ensureTimer];
    }
    return self;
}

- (SmalltalkIslandButton *)buttonWithTitle:(NSString *)title action:(NSString *)action fill:(NSColor *)fill {
    SmalltalkIslandButton *button = [[SmalltalkIslandButton alloc] initWithFrame:NSZeroRect];
    button.title = title;
    button.islandAction = action;
    button.bordered = NO;
    button.font = [NSFont systemFontOfSize:11 weight:NSFontWeightSemibold];
    button.contentTintColor = NSColor.whiteColor;
    button.wantsLayer = YES;
    button.layer.cornerRadius = 10.0;
    button.layer.cornerCurve = kCACornerCurveContinuous;
    button.layer.backgroundColor = fill.CGColor;
    button.target = self;
    button.action = @selector(buttonPressed:);
    return button;
}

- (BOOL)acceptsFirstMouse:(NSEvent *)event { return YES; }

- (void)mouseDown:(NSEvent *)event {
    if ([self.model.state isEqualToString:@"recording_compact"] || [self.model.state isEqualToString:@"recording_expanded"]) {
        SmalltalkSendAction(@"toggle_expanded");
    }
}

- (void)dealloc {
    [_timer invalidate];
}

- (void)buttonPressed:(SmalltalkIslandButton *)sender {
    SmalltalkSendAction(sender.islandAction);
}

- (void)ensureTimer {
    if (_timer) {
        return;
    }
    _timer = [NSTimer timerWithTimeInterval:1.0 target:self selector:@selector(tick:) userInfo:nil repeats:YES];
    [[NSRunLoop mainRunLoop] addTimer:_timer forMode:NSRunLoopCommonModes];
}

- (void)tick:(NSTimer *)timer {
    [self refreshText];
}

- (void)updateWithViewModel:(SmalltalkIslandViewModel *)model expanded:(BOOL)expanded reduceMotion:(BOOL)reduceMotion {
    self.model = model;
    self.expanded = expanded;
    self.reduceMotion = reduceMotion;
    self.modelReceivedAtMs = SmalltalkNowMillis();
    [self refreshText];
    [self updatePulse];
    [self setNeedsLayout:YES];
}

- (long long)displayElapsedMs {
    BOOL live = [self.model.state isEqualToString:@"recording_compact"]
        || [self.model.state isEqualToString:@"recording_expanded"]
        || [self.model.state isEqualToString:@"processing"]
        || [self.model.state isEqualToString:@"starting"];
    if (!live) {
        return self.model.elapsedMs;
    }
    return self.model.elapsedMs + MAX(0, SmalltalkNowMillis() - self.modelReceivedAtMs);
}

- (void)refreshText {
    NSString *state = self.model.state;
    BOOL processing = [state isEqualToString:@"processing"];
    BOOL stopped = [state isEqualToString:@"stopped_toast"];
    BOOL error = [state isEqualToString:@"error"];
    BOOL starting = [state isEqualToString:@"starting"];

    if (error) {
        self.statusLabel.stringValue = @"Capture issue";
    } else if (stopped) {
        self.statusLabel.stringValue = @"Session saved";
    } else if (processing) {
        self.statusLabel.stringValue = @"Saving session";
    } else if (starting) {
        self.statusLabel.stringValue = @"Starting";
    } else {
        self.statusLabel.stringValue = @"Recording";
    }
    self.elapsedLabel.stringValue = SmalltalkFormatElapsed([self displayElapsedMs]);

    if (error) {
        self.currentLabel.stringValue = self.model.lastError.length > 0 ? self.model.lastError : @"Open Smalltalk for details";
        self.lastLabel.stringValue = @"The island will stay small until capture recovers.";
        self.factsLabel.stringValue = @"";
    } else if (processing) {
        self.currentLabel.stringValue = @"Saving evidence locally";
        self.lastLabel.stringValue = @"Building export summary";
        self.factsLabel.stringValue = [NSString stringWithFormat:@"Frames: %lld", self.model.frameCount];
    } else if (self.model.isSensitive) {
        self.currentLabel.stringValue = @"Current: Private surface";
        self.lastLabel.stringValue = [NSString stringWithFormat:@"Last capture: %@", SmalltalkHumanizeTrigger(self.model.lastTrigger)];
        self.factsLabel.stringValue = [NSString stringWithFormat:@"Frames: %lld - Privacy: %@", self.model.frameCount, self.model.privacyLabel.length > 0 ? self.model.privacyLabel : @"private"];
    } else {
        NSString *surface = self.model.currentApp.length > 0 ? self.model.currentApp : @"Waiting for first surface";
        if (self.model.currentWindow.length > 0) {
            surface = [NSString stringWithFormat:@"%@ - %@", surface, self.model.currentWindow];
        }
        self.currentLabel.stringValue = [NSString stringWithFormat:@"Current: %@", surface];
        self.lastLabel.stringValue = [NSString stringWithFormat:@"Last capture: %@", SmalltalkHumanizeTrigger(self.model.lastTrigger)];
        NSString *text = self.model.currentSurfaceKind.length > 0 ? self.model.currentSurfaceKind : @"unknown";
        NSString *privacy = self.model.privacyLabel.length > 0 ? self.model.privacyLabel : @"normal";
        self.factsLabel.stringValue = [NSString stringWithFormat:@"Frames: %lld - Text: %@ - Privacy: %@", self.model.frameCount, text, privacy];
    }
}

- (void)updatePulse {
    BOOL recording = [self.model.state isEqualToString:@"recording_compact"] || [self.model.state isEqualToString:@"recording_expanded"] || [self.model.state isEqualToString:@"starting"];
    [self.dotView.layer removeAnimationForKey:@"smalltalk-pulse"];
    if (!recording || self.reduceMotion) {
        self.dotView.layer.opacity = recording ? 1.0 : 0.0;
        return;
    }
    CABasicAnimation *animation = [CABasicAnimation animationWithKeyPath:@"opacity"];
    animation.fromValue = @1.0;
    animation.toValue = @0.35;
    animation.duration = 0.85;
    animation.autoreverses = YES;
    animation.repeatCount = HUGE_VALF;
    [self.dotView.layer addAnimation:animation forKey:@"smalltalk-pulse"];
}

- (void)updateShapeMaskCompact:(BOOL)isCompact {
    CGFloat width = self.bounds.size.width;
    CGFloat height = self.bounds.size.height;
    CGFloat radius = isCompact ? 17.0 : 28.0;
    radius = MIN(radius, height / 2.0);

    CGMutablePathRef path = CGPathCreateMutable();
    CGPathMoveToPoint(path, NULL, 0.0, height);
    CGPathAddLineToPoint(path, NULL, width, height);
    CGPathAddLineToPoint(path, NULL, width, radius);
    CGPathAddQuadCurveToPoint(path, NULL, width, 0.0, width - radius, 0.0);
    CGPathAddLineToPoint(path, NULL, radius, 0.0);
    CGPathAddQuadCurveToPoint(path, NULL, 0.0, 0.0, 0.0, radius);
    CGPathAddLineToPoint(path, NULL, 0.0, height);
    CGPathCloseSubpath(path);

    self.shapeMaskLayer.frame = self.bounds;
    self.shapeMaskLayer.path = path;
    CGPathRelease(path);
}

- (void)layout {
    [super layout];
    CGFloat width = self.bounds.size.width;
    CGFloat height = self.bounds.size.height;
    BOOL isCompact = height <= 44.0 && !self.expanded;
    BOOL processing = [self.model.state isEqualToString:@"processing"];
    BOOL stopped = [self.model.state isEqualToString:@"stopped_toast"];
    BOOL error = [self.model.state isEqualToString:@"error"];

    [self updateShapeMaskCompact:isCompact];
    self.layer.backgroundColor = [[NSColor colorWithWhite:0.0 alpha:isCompact ? 0.96 : 0.94] CGColor];

    self.dotView.hidden = processing || stopped || error;
    self.spinner.hidden = !processing;
    processing ? [self.spinner startAnimation:nil] : [self.spinner stopAnimation:nil];

    self.statusLabel.hidden = isCompact && !processing && !error && !stopped;
    self.elapsedLabel.hidden = NO;
    self.currentLabel.hidden = isCompact || stopped;
    self.lastLabel.hidden = isCompact || stopped;
    self.factsLabel.hidden = isCompact || stopped;
    self.openButton.hidden = isCompact && !error;
    self.resumeButton.hidden = isCompact || processing || stopped || error;
    self.dismissButton.hidden = isCompact || !error;
    self.stopButton.hidden = processing || stopped || error;

    if (isCompact) {
        CGFloat timerWidth = 48.0;
        CGFloat stopSize = 20.0;
        CGFloat dotSize = 7.0;
        CGFloat spacing = 9.0;
        CGFloat groupWidth = dotSize + spacing + timerWidth + spacing + stopSize;
        CGFloat x = floor((width - groupWidth) / 2.0);
        CGFloat centerY = floor((height - dotSize) / 2.0);

        self.dotView.frame = NSMakeRect(x, centerY, dotSize, dotSize);
        self.spinner.frame = NSMakeRect(x - 4.0, floor((height - 16.0) / 2.0), 16.0, 16.0);
        self.elapsedLabel.frame = NSMakeRect(x + dotSize + spacing, floor((height - 16.0) / 2.0), timerWidth, 16.0);
        self.stopButton.title = @"";
        self.stopButton.image = [NSImage imageWithSystemSymbolName:@"stop.fill" accessibilityDescription:@"Stop capture"];
        [self.stopButton.image setTemplate:YES];
        self.stopButton.imagePosition = NSImageOnly;
        self.stopButton.imageScaling = NSImageScaleProportionallyDown;
        self.stopButton.layer.cornerRadius = stopSize / 2.0;
        self.stopButton.layer.backgroundColor = [[NSColor colorWithRed:0.78 green:0.14 blue:0.12 alpha:0.82] CGColor];
        if (processing || stopped) {
            self.elapsedLabel.hidden = YES;
            self.statusLabel.hidden = NO;
            self.statusLabel.stringValue = processing ? @"Saving" : @"Saved";
            CGFloat labelWidth = 58.0;
            if (processing) {
                CGFloat totalWidth = 16.0 + 8.0 + labelWidth;
                CGFloat startX = floor((width - totalWidth) / 2.0);
                self.spinner.frame = NSMakeRect(startX, floor((height - 16.0) / 2.0), 16.0, 16.0);
                self.statusLabel.frame = NSMakeRect(startX + 24.0, floor((height - 16.0) / 2.0), labelWidth, 16.0);
            } else {
                self.statusLabel.frame = NSMakeRect(floor((width - labelWidth) / 2.0), floor((height - 16.0) / 2.0), labelWidth, 16.0);
            }
        } else if (error) {
            self.elapsedLabel.hidden = YES;
            self.openButton.hidden = NO;
            self.openButton.title = @"Open";
            self.statusLabel.hidden = NO;
            self.statusLabel.stringValue = @"!";
            self.statusLabel.frame = NSMakeRect(x, floor((height - 16.0) / 2.0), 12.0, 16.0);
            self.openButton.frame = NSMakeRect(width - 62.0, floor((height - 22.0) / 2.0), 48.0, 22.0);
        } else {
            self.stopButton.frame = NSMakeRect(x + dotSize + spacing + timerWidth + spacing, floor((height - stopSize) / 2.0), stopSize, stopSize);
        }
        return;
    }

    self.dotView.frame = NSMakeRect(20, height - 29, 7, 7);
    self.spinner.frame = NSMakeRect(14, height - 33, 16, 16);
    self.statusLabel.hidden = NO;
    self.statusLabel.frame = NSMakeRect(36, height - 35, 160, 20);
    self.elapsedLabel.frame = NSMakeRect(width - 82, height - 34, 52, 18);
    self.stopButton.title = @"Stop";
    self.stopButton.image = nil;
    self.stopButton.imagePosition = NSNoImage;
    self.stopButton.font = [NSFont systemFontOfSize:12 weight:NSFontWeightSemibold];
    self.stopButton.layer.cornerRadius = 13.0;
    self.stopButton.layer.backgroundColor = [[NSColor colorWithRed:0.78 green:0.14 blue:0.12 alpha:0.94] CGColor];
    self.stopButton.frame = NSMakeRect(width - 82, height - 68, 62, 26);
    self.currentLabel.frame = NSMakeRect(20, height - 74, width - 118, 20);
    self.lastLabel.frame = NSMakeRect(20, height - 99, width - 40, 18);
    self.factsLabel.frame = NSMakeRect(20, height - 122, width - 40, 18);
    self.openButton.title = @"Open Smalltalk";
    self.openButton.frame = NSMakeRect(20, 18, 116, 28);
    self.resumeButton.frame = NSMakeRect(146, 18, 92, 28);
    self.dismissButton.frame = NSMakeRect(146, 18, 82, 28);
}
@end

@interface SmalltalkSessionIslandController : NSObject
+ (instancetype)shared;
- (void)initializeIfNeeded;
- (void)updateWithJSONString:(NSString *)json;
- (void)show;
- (void)hide;
- (void)setExpanded:(BOOL)expanded;
- (void)reposition;
- (void)shutdown;
@end

@interface SmalltalkSessionIslandController ()
@property(nonatomic, strong) SmalltalkIslandPanel *panel;
@property(nonatomic, strong) SmalltalkIslandRootView *rootView;
@property(nonatomic, strong) SmalltalkIslandViewModel *viewModel;
@property(nonatomic) BOOL isExpanded;
@property(nonatomic) BOOL reduceMotion;
@property(nonatomic) BOOL visible;
@end

@implementation SmalltalkSessionIslandController
+ (instancetype)shared {
    static SmalltalkSessionIslandController *controller = nil;
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        controller = [[SmalltalkSessionIslandController alloc] init];
    });
    return controller;
}

- (void)initializeIfNeeded {
    if (self.panel) {
        return;
    }

    self.viewModel = [[SmalltalkIslandViewModel alloc] init];
    self.reduceMotion = NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion;
    self.rootView = [[SmalltalkIslandRootView alloc] initWithFrame:NSMakeRect(0, 0, kVirtualCompactWidth, kCompactHeight)];

    self.panel = [[SmalltalkIslandPanel alloc]
        initWithContentRect:NSMakeRect(0, 0, kVirtualCompactWidth, kCompactHeight)
                  styleMask:(NSWindowStyleMaskBorderless | NSWindowStyleMaskNonactivatingPanel)
                    backing:NSBackingStoreBuffered
                      defer:NO];
    self.panel.floatingPanel = YES;
    self.panel.hidesOnDeactivate = NO;
    self.panel.releasedWhenClosed = NO;
    self.panel.opaque = NO;
    self.panel.backgroundColor = NSColor.clearColor;
    self.panel.hasShadow = NO;
    self.panel.level = NSStatusWindowLevel;
    self.panel.collectionBehavior = NSWindowCollectionBehaviorCanJoinAllSpaces
        | NSWindowCollectionBehaviorFullScreenAuxiliary
        | NSWindowCollectionBehaviorStationary
        | NSWindowCollectionBehaviorIgnoresCycle;
    self.panel.contentView = self.rootView;
    [self.rootView updateWithViewModel:self.viewModel expanded:NO reduceMotion:self.reduceMotion];
    [self.panel setFrame:[self targetFrame] display:NO];

    NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
    [center addObserver:self selector:@selector(screenParametersDidChange:) name:NSApplicationDidChangeScreenParametersNotification object:nil];
    [center addObserver:self selector:@selector(accessibilityOptionsDidChange:) name:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:nil];

    NSLog(@"[session_island] init");
}

- (NSSize)targetSizeForScreen:(NSScreen *)screen {
    if ([self.viewModel.state isEqualToString:@"stopped_toast"]) {
        return NSMakeSize(kToastWidth, kToastHeight);
    }
    if (self.isExpanded) {
        return NSMakeSize(kExpandedWidth, kExpandedHeight);
    }
    return SmalltalkCompactSizeForScreen(screen);
}

- (NSRect)targetFrame {
    NSScreen *screen = SmalltalkTargetScreen();
    if (!screen) {
        return NSMakeRect(0, 0, kVirtualCompactWidth, kCompactHeight);
    }
    return SmalltalkIslandFrame(screen, [self targetSizeForScreen:screen]);
}

- (SmalltalkIslandViewModel *)viewModelFromJSON:(NSString *)json {
    SmalltalkIslandViewModel *model = [[SmalltalkIslandViewModel alloc] init];
    NSData *data = [json dataUsingEncoding:NSUTF8StringEncoding];
    if (!data) {
        return model;
    }
    NSError *error = nil;
    id object = [NSJSONSerialization JSONObjectWithData:data options:0 error:&error];
    if (![object isKindOfClass:NSDictionary.class]) {
        if (error) {
            NSLog(@"[session_island] json parse error: %@", error.localizedDescription);
        }
        return model;
    }
    NSDictionary *dict = (NSDictionary *)object;
    model.state = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"state"]));
    model.sessionId = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"session_id"]));
    model.elapsedMs = [dict[@"elapsed_ms"] respondsToSelector:@selector(longLongValue)] ? [dict[@"elapsed_ms"] longLongValue] : 0;
    model.frameCount = [dict[@"frame_count"] respondsToSelector:@selector(longLongValue)] ? [dict[@"frame_count"] longLongValue] : 0;
    model.currentApp = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"current_app"]));
    model.currentWindow = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"current_window"]));
    model.currentSurfaceKind = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"current_surface_kind"]));
    model.lastTrigger = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"last_trigger"]));
    model.lastError = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"last_error"]));
    model.privacyLabel = SmalltalkCleanLabel(SmalltalkStringOrEmpty(dict[@"privacy_label"]));
    model.isSensitive = [dict[@"is_sensitive"] respondsToSelector:@selector(boolValue)] ? [dict[@"is_sensitive"] boolValue] : NO;
    if (model.state.length == 0) {
        model.state = @"hidden";
    }
    return model;
}

- (void)updateWithJSONString:(NSString *)json {
    [self initializeIfNeeded];
    self.viewModel = [self viewModelFromJSON:json ?: @"{}"];
    if ([self.viewModel.state isEqualToString:@"hidden"]) {
        [self hide];
        return;
    }
    if ([self.viewModel.state isEqualToString:@"starting"]
        || [self.viewModel.state isEqualToString:@"processing"]
        || [self.viewModel.state isEqualToString:@"stopped_toast"]) {
        _isExpanded = NO;
    }
    [self.rootView updateWithViewModel:self.viewModel expanded:self.isExpanded reduceMotion:self.reduceMotion];
    [self show];
    [self reposition];
    NSLog(@"[session_island] update state=%@ elapsed=%lld", self.viewModel.state, self.viewModel.elapsedMs);

    if ([self.viewModel.state isEqualToString:@"stopped_toast"]) {
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(1.6 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
            if ([self.viewModel.state isEqualToString:@"stopped_toast"]) {
                [self hide];
            }
        });
    }
}

- (void)show {
    [self initializeIfNeeded];
    if (!self.visible) {
        [self.panel setFrame:[self targetFrame] display:YES];
        [self.panel orderFrontRegardless];
        self.visible = YES;
        NSLog(@"[session_island] show");
    }
}

- (void)hide {
    [self initializeIfNeeded];
    _isExpanded = NO;
    self.visible = NO;
    [self.panel orderOut:nil];
    NSLog(@"[session_island] hide");
}

- (void)setExpanded:(BOOL)expanded {
    [self initializeIfNeeded];
    if ([self.viewModel.state isEqualToString:@"hidden"]
        || [self.viewModel.state isEqualToString:@"processing"]
        || [self.viewModel.state isEqualToString:@"stopped_toast"]) {
        expanded = NO;
    }
    _isExpanded = expanded;
    [self.rootView updateWithViewModel:self.viewModel expanded:_isExpanded reduceMotion:self.reduceMotion];
    [self reposition];
}

- (void)reposition {
    [self initializeIfNeeded];
    NSRect target = [self targetFrame];
    NSTimeInterval duration = self.reduceMotion ? 0.12 : (self.isExpanded ? 0.42 : 0.24);
    [NSAnimationContext runAnimationGroup:^(NSAnimationContext *context) {
        context.duration = duration;
        context.timingFunction = [CAMediaTimingFunction functionWithControlPoints:0.22 :1.0 :0.36 :1.0];
        [self.panel.animator setFrame:target display:YES];
    } completionHandler:nil];
}

- (void)shutdown {
    [NSNotificationCenter.defaultCenter removeObserver:self];
    [self.panel orderOut:nil];
    self.panel = nil;
    self.rootView = nil;
    self.viewModel = nil;
    self.visible = NO;
    NSLog(@"[session_island] shutdown");
}

- (void)screenParametersDidChange:(NSNotification *)notification {
    [self reposition];
}

- (void)accessibilityOptionsDidChange:(NSNotification *)notification {
    self.reduceMotion = NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion;
    [self.rootView updateWithViewModel:self.viewModel expanded:self.isExpanded reduceMotion:self.reduceMotion];
}
@end

void smalltalk_island_init(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [[SmalltalkSessionIslandController shared] initializeIfNeeded];
    });
}

void smalltalk_island_set_action_callback(SmalltalkIslandActionCallback callback) {
    gActionCallback = callback;
}

void smalltalk_island_update_json(const char *json) {
    NSString *copied = json ? [NSString stringWithUTF8String:json] : @"{}";
    dispatch_async(dispatch_get_main_queue(), ^{
        [[SmalltalkSessionIslandController shared] updateWithJSONString:copied ?: @"{}"];
    });
}

void smalltalk_island_show(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [[SmalltalkSessionIslandController shared] show];
    });
}

void smalltalk_island_hide(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [[SmalltalkSessionIslandController shared] hide];
    });
}

void smalltalk_island_set_expanded(bool expanded) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [[SmalltalkSessionIslandController shared] setExpanded:expanded ? YES : NO];
    });
}

void smalltalk_island_reposition(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [[SmalltalkSessionIslandController shared] reposition];
    });
}

void smalltalk_island_shutdown(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [[SmalltalkSessionIslandController shared] shutdown];
    });
}
