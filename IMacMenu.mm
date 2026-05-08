#include "IMacMenu.h"
#include <QString>
#include <QWidget>

#ifdef Q_OS_MACOS
#import <AppKit/AppKit.h>
#endif

void DisableMacServicesMenu()
{
#ifdef Q_OS_MACOS
    [NSApp setServicesMenu:nil];

    NSMenu* pnsmMainMenu = [NSApp mainMenu];
    NSMenuItem* pnsmiAppMenuItem = [pnsmMainMenu itemAtIndex:0];
    NSMenu* pnsmAppMenu = [pnsmiAppMenuItem submenu];
    if (pnsmAppMenu == nil)
        return;

    NSArray<NSMenuItem*>* pnsaItems = [[pnsmAppMenu itemArray] copy];
    for (NSMenuItem* pnsmiItem in pnsaItems)
    {
        if ([[pnsmiItem title] isEqualToString:@"Services"])
        {
            [pnsmAppMenu removeItem:pnsmiItem];
            break;
        }
    }
#endif
}

void ApplyMacCenteredWindowTitle(QWidget* pWidget, const QString & krqstrTitle)
{
#ifdef Q_OS_MACOS
    if (pWidget == nullptr)
        return;

    NSView* pnsvQtView = reinterpret_cast<NSView*>(pWidget->winId());
    NSWindow* pnswWindow = [pnsvQtView window];
    NSView* pnsvFrameView = [[pnswWindow contentView] superview];
    if (pnswWindow == nil || pnsvFrameView == nil)
        return;

    [pnswWindow setTitleVisibility:NSWindowTitleHidden];

    NSString* pnssIdentifier = @"InviskaCenteredTitleLabel";
    for (NSView* pnsvSubview in [pnsvFrameView subviews])
    {
        if ([[pnsvSubview accessibilityIdentifier] isEqualToString:pnssIdentifier])
            [pnsvSubview removeFromSuperview];
    }

    QByteArray qbaTitle = krqstrTitle.toUtf8();
    NSString* pnssTitle = [NSString stringWithUTF8String:qbaTitle.constData()];
    NSTextField* pnstfTitle = [NSTextField labelWithString:pnssTitle];
    [pnstfTitle setAccessibilityIdentifier:pnssIdentifier];
    [pnstfTitle setAlignment:NSTextAlignmentCenter];
    [pnstfTitle setFont:[NSFont boldSystemFontOfSize:[NSFont systemFontSize]]];
    [pnstfTitle setTextColor:[NSColor secondaryLabelColor]];
    [pnstfTitle setTranslatesAutoresizingMaskIntoConstraints:NO];
    [pnsvFrameView addSubview:pnstfTitle];

    [NSLayoutConstraint activateConstraints:@[
        [[pnstfTitle centerXAnchor] constraintEqualToAnchor:[pnsvFrameView centerXAnchor]],
        [[pnstfTitle topAnchor] constraintEqualToAnchor:[pnsvFrameView topAnchor] constant:6.0],
        [[pnstfTitle widthAnchor] constraintLessThanOrEqualToAnchor:[pnsvFrameView widthAnchor] multiplier:0.7],
        [[pnstfTitle heightAnchor] constraintEqualToConstant:20.0]
    ]];
#else
    Q_UNUSED(pWidget);
    Q_UNUSED(krqstrTitle);
#endif
}
