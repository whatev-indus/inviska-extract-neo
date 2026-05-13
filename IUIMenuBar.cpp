#include <QtWidgets>
#include <QAction>
#include "IUIMenuBar.h"
#include "IUIMainWindow.h"
#include "IUIExtract.h"


IUIMenuBar::IUIMenuBar(IUIMainWindow* pmwMainWindow) : IComUIMenuBarBase(pmwMainWindow)
{
    CreateActions();
    InitialiseMenuBar();
}


void IUIMenuBar::CreateActions()
{
    IUIExtract* & rpuiextExtractUI = m_pmwMainWindow->GetExtractUI();

    // File menu actions
    m_pqactOpenFiles = new QAction(tr("&Add Files..."), m_pmwMainWindow);
    m_pqactOpenFiles->setIcon(m_pmwMainWindow->style()->standardIcon(QStyle::SP_DirOpenIcon));
    m_pqactOpenFiles->setShortcut(QKeySequence::Open);
    QObject::connect(m_pqactOpenFiles, &QAction::triggered, rpuiextExtractUI, &IUIExtract::OpenFilesDialog);

    m_pqactOpenDir = new QAction(tr("&Add Directory..."), m_pmwMainWindow);
    m_pqactOpenDir->setIcon(m_pmwMainWindow->style()->standardIcon(QStyle::SP_DirOpenIcon));
    QObject::connect(m_pqactOpenDir, &QAction::triggered, rpuiextExtractUI, &IUIExtract::OpenDirDialog);

    IComUIMenuBarBase::CreateActions();
}


void IUIMenuBar::InitialiseMenuBar()
{
    m_pqmenuFile = m_pmwMainWindow->menuBar()->addMenu(tr("&File"));
    m_pqmenuFile->addAction(m_pqactOpenFiles);
    m_pqmenuFile->addAction(m_pqactOpenDir);
    m_pqmenuFile->addAction(m_pqactPreferences);
    m_pqmenuFile->addSeparator();
    m_pqmenuFile->addAction(m_pqactExit);

    IComUIMenuBarBase::InitialiseMenuBar();
}
