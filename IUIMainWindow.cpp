#include <QtWidgets>
#include "IUIMainWindow.h"
#include "IUIExtract.h"
#include "IUIMenuBar.h"
#include "IMacMenu.h"


IUIMainWindow::IUIMainWindow(QSettings & rqsetSettings, IComSysSingleInstance & rsnglSingleInstance) : IComUIMainWinBase(rqsetSettings, rsnglSingleInstance)
{
    m_puiextExtractUI = new IUIExtract(this);
    setCentralWidget(m_puiextExtractUI);

    m_puimbMenuBar = new IUIMenuBar(this);

    #ifdef Q_OS_MACOS
    QTimer::singleShot(0, this, [this]() {
        DisableMacServicesMenu();
        ApplyMacCenteredWindowTitle(this, windowTitle());
    });
    #endif
}



IUIMainWindow::~IUIMainWindow()
{

}


void IUIMainWindow::ProcessCommandLineParameters()
{
    m_puiextExtractUI->ProcessCommandLineParameters();
}
