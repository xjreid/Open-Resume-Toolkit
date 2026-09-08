#import <AppKit/AppKit.h>
#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <pthread.h>

bool ort_macos_install_termination(bool (*callback)(void));
bool ort_macos_reply_termination(bool approve);
static unsigned requests = 0;
static unsigned callbacks = 0;
static bool cancelled = false;
static bool approving = false;

static void *reply_from_worker(void *unused) {
    (void)unused;
    @autoreleasepool {
        assert(!ort_macos_reply_termination(true));
    }
    return NULL;
}

@interface ProbeDelegate : NSObject <NSApplicationDelegate>
@end
@implementation ProbeDelegate
- (void)applicationWillTerminate:(NSNotification *)notification {
    (void)notification;
    assert(approving && cancelled && requests == 3 && callbacks == 3);
    puts("PASS: AppKit terminate deferred, cancel and callback failure kept app alive, repeat coalesced, explicit approval reached applicationWillTerminate.");
    fflush(stdout);
}
@end

@interface ExistingDelegate : NSObject <NSApplicationDelegate>
@end
@implementation ExistingDelegate
- (NSApplicationTerminateReply)applicationShouldTerminate:(NSApplication *)sender {
    (void)sender;
    return NSTerminateCancel;
}
@end

static bool close_request(void) {
    callbacks++;
    fprintf(stderr, "callback %u\n", callbacks);
    assert([NSThread isMainThread]);
    if (requests == 1) {
        // Repeated termination should not issue another request while pending.
        assert([NSApp.delegate applicationShouldTerminate:NSApp] == NSTerminateLater);
        ProbeDelegate *other = [ProbeDelegate new];
        assert([other applicationShouldTerminate:NSApp] == NSTerminateCancel);
        assert(callbacks == 1);
        // A worker thread cannot approve or consume the outstanding request.
        pthread_t worker;
        assert(pthread_create(&worker, NULL, reply_from_worker, NULL) == 0);
        assert(pthread_join(worker, NULL) == 0);
        assert(ort_macos_reply_termination(false));
        assert(!ort_macos_reply_termination(false));
        cancelled = true;
        dispatch_async(dispatch_get_main_queue(), ^{
            assert(cancelled);
            requests++;
            fprintf(stderr, "second terminate\n");
            [NSApp terminate:nil];
        });
    } else if (requests == 2) {
        assert(requests == 2 && cancelled);
        dispatch_async(dispatch_get_main_queue(), ^{
            // Returning false must cancel the native request before this runs.
            assert(!ort_macos_reply_termination(true));
            requests++;
            [NSApp terminate:nil];
        });
        return false;
    } else {
        assert(requests == 3 && cancelled);
        approving = true;
        assert(ort_macos_reply_termination(true));
    }
    return true;
}

int main(void) {
    if (getuid() == 0 || geteuid() == 0) return 65;
    alarm(10);
    @autoreleasepool {
        NSApplication *app = [NSApplication sharedApplication];
        [app setActivationPolicy:NSApplicationActivationPolicyProhibited];
        ExistingDelegate *existing = [ExistingDelegate new];
        app.delegate = existing;
        assert(!ort_macos_install_termination(close_request));
        assert(!ort_macos_reply_termination(true));
        ProbeDelegate *delegate = [ProbeDelegate new];
        app.delegate = delegate;
        assert(ort_macos_install_termination(close_request));
        assert(!ort_macos_install_termination(close_request));
        dispatch_async(dispatch_get_main_queue(), ^{
            requests++;
            fprintf(stderr, "first terminate\n");
            [app terminate:nil];
        });
        fprintf(stderr, "starting app run\n");
        [app run];
    }
    return 66; // AppKit approval should exit, not fall through the run loop.
}
