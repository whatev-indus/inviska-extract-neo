#ifndef IComUIMenuBarBase_h
#define IComUIMenuBarBase_h

#include <QObject>
class IUIMainWindow;
class QMenu;
class QAction;


class IComUIMenuBarBase : public QObject
{
    Q_OBJECT

protected:
    // Main window
    IUIMainWindow*              m_pmwMainWindow;

    // File Menu
    QMenu*                      m_pqmenuFile;
    QAction*                    m_pqactPreferences;
    QAction*                    m_pqactExit;

protected:
    IComUIMenuBarBase(IUIMainWindow* pmwMainWindow);

protected:
    // Creates actions present in all applications
    virtual void CreateActions();

    // Creates menus and adds actions
    virtual void InitialiseMenuBar();

private slots:
    // Menu items action functions
    void ShowPreferencesDialog();
};

#endif // IComUIMenuBarBase_h
