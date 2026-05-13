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
    m_pqactPreferences->setIcon(QIcon(":/Resources/Settings.png"));
    m_pqactPreferences->setShortcut(QKeySequence::Preferences);
    m_pqactPreferences->setToolTip(tr("Change application settings"));
    QObject::connect(m_pqactPreferences, &QAction::triggered, this, &IComUIMenuBarBase::ShowPreferencesDialog);

    m_pqactExit = new QAction(tr("E&xit"), m_pmwMainWindow);
    m_pqactExit->setShortcut(tr("Ctrl+Q", "Quit/Exit application"));
    QObject::connect(m_pqactExit, &QAction::triggered, m_pmwMainWindow, &QWidget::close);
}


void IComUIMenuBarBase::InitialiseMenuBar()
{
}


void IComUIMenuBarBase::ShowPreferencesDialog()
{
    new IDlgPreferences(m_pmwMainWindow);
}
