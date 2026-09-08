#import <AppKit/AppKit.h>
#import <objc/runtime.h>
#include <stdbool.h>

// Main-thread-only process-lifetime bridge. Adds only an absent delegate method;
// never replaces the delegate or swizzles an existing implementation.
static bool (*request_close)(void) = NULL;
static bool pending = false;
static Class installed_class = Nil;

bool ort_macos_reply_termination(bool approve) {
    if (![NSThread isMainThread] || !pending) return false;
    pending = false; // Clear before AppKit can reenter the delegate.
    [NSApp replyToApplicationShouldTerminate:approve];
    return true;
}

// Obtain the ABI encoding from an Objective-C declaration rather than assuming
// NSUInteger width. The helper also schedules the wakeup in AppKit modal modes.
@interface ORTTerminationSignature : NSObject
- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender;
+ (void)deliverTerminationRequest;
@end
@implementation ORTTerminationSignature
- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender {
    (void)sender;
    return NSTerminateCancel;
}
+ (void)deliverTerminationRequest {
    if (pending && !request_close()) (void)ort_macos_reply_termination(false);
}
@end

static NSApplicationTerminateReply should_terminate(id delegate, SEL selector, NSApplication *sender) {
    (void)selector;
    if (![NSThread isMainThread] || sender != NSApp || sender.delegate != delegate
        || object_getClass(delegate) != installed_class || request_close == NULL)
        return NSTerminateCancel;
    if (!pending) {
        pending = true;
        // NSTerminateLater can spin a nested modal run loop while terminate:
        // is still on the stack. dispatch_async(main) cannot reenter a running
        // main-queue block, so use an AppKit run-loop selector instead.
        [ORTTerminationSignature performSelector:@selector(deliverTerminationRequest)
            withObject:nil afterDelay:0 inModes:@[NSDefaultRunLoopMode,
                NSModalPanelRunLoopMode, NSEventTrackingRunLoopMode]];
    }
    return NSTerminateLater;
}

bool ort_macos_install_termination(bool (*callback)(void)) {
    if (![NSThread isMainThread] || callback == NULL || installed_class != Nil || NSApp == nil)
        return false;
    id delegate = NSApp.delegate;
    Class target = object_getClass(delegate);
    SEL selector = @selector(applicationShouldTerminate:);
    if (target == Nil || class_getInstanceMethod(target, selector) != NULL) return false;
    Method signature = class_getInstanceMethod([ORTTerminationSignature class], selector);
    if (signature == NULL) return false;
    request_close = callback;
    installed_class = target;
    if (!class_addMethod(target, selector, (IMP)should_terminate, method_getTypeEncoding(signature))) {
        installed_class = Nil;
        request_close = NULL;
        return false;
    }
    return true;
}
