#ifndef IMacMenu_h
#define IMacMenu_h

#include <QtGlobal>

class QString;
class QWidget;

#ifdef Q_OS_MACOS
void DisableMacServicesMenu();
void ApplyMacCenteredWindowTitle(QWidget* pWidget, const QString & krqstrTitle);
#else
inline void DisableMacServicesMenu() {}
inline void ApplyMacCenteredWindowTitle(QWidget*, const QString &) {}
#endif

#endif // IMacMenu_h
