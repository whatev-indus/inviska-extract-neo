#include <QtWidgets>
#include <QAction>
#include "IComUIMenuBarBase.h"
#include "IUIMainWindow.h"
#include "IDlgPreferences.h"


IComUIMenuBarBase::IComUIMenuBarBase(IUIMainWindow* pmwMainWindow) : QObject(pmwMainWindow)
{
    m_pmwMainWindow = pmwMainWindow;
}


void IComUIMenuBarBase::CreateActions()
{
    m_pqactPreferences = new QAction(tr("&Preferences..."), m_pmwMainWindow);
    m_pqactPreferences->setIcon(m_pmwMainWindow->style()->standardIcon(QStyle::SP_FileDialogDetailedView));
    m_pqactPreferences->setShortcut(QKeySequence::Preferences);
    m_pqactPreferences->setToolTip(tr("Change application settings"));
    QObject::connect(m_pqactPreferences, SIGNAL(triggered()), this, SLOT(ShowPreferencesDialog()));

    m_pqactExit = new QAction(tr("E&xit"), m_pmwMainWindow);
    m_pqactExit->setShortcut(tr("Ctrl+Q", "Quit/Exit application"));
    QObject::connect(m_pqactExit, SIGNAL(triggered()), m_pmwMainWindow, SLOT(close()));
}


void IComUIMenuBarBase::InitialiseMenuBar()
{
}


void IComUIMenuBarBase::ShowPreferencesDialog()
{
    new IDlgPreferences(m_pmwMainWindow);
}
