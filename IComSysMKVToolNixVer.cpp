#include <QtWidgets>
#include "IUIMainWindow.h"
#include "IComSysMKVToolNixVer.h"
#include "IComSysMKVToolNix.h"


IComSysMKVToolNixVer::IComSysMKVToolNixVer(IUIMainWindow* pmwMainWindow, IComSysMKVToolNix* pimkvMKVToolNix)
{
    m_pmwMainWindow     = pmwMainWindow;
    m_pimkvMKVToolNix   = pimkvMKVToolNix;

    connect(&m_qprocMKVMerge, SIGNAL(readyReadStandardOutput()),             this, SLOT(MKVMergeOutputText()));
    connect(&m_qprocMKVMerge, SIGNAL(readyReadStandardError()),              this, SLOT(MKVMergeErrorText()));
    connect(&m_qprocMKVMerge, SIGNAL(finished(int, QProcess::ExitStatus)),   this, SLOT(MKVMergeFinished(int, QProcess::ExitStatus)));
    connect(&m_qprocMKVMerge, SIGNAL(errorOccurred(QProcess::ProcessError)), this, SLOT(MKVMergeError(QProcess::ProcessError)));
}


void IComSysMKVToolNixVer::DetermineMKVToolNixVersion(const QString & krqstrMKVMergePath)
{
    m_qstrVersionString.clear();

    m_qprocMKVMerge.setProgram(krqstrMKVMergePath);
    m_qprocMKVMerge.setArguments(QStringList("--version"));
    m_qprocMKVMerge.start();
}


void IComSysMKVToolNixVer::MKVMergeOutputText()
{
    QByteArray qbaData = m_qprocMKVMerge.readAllStandardOutput();
    m_qstrVersionString += QString::fromUtf8(qbaData);
}


void IComSysMKVToolNixVer::MKVMergeFinished(const int kiExitCode, const QProcess::ExitStatus kqpesExitStatus)
{
    if (kqpesExitStatus == QProcess::CrashExit)
    {
        QMessageBox::critical(m_pmwMainWindow, tr("Error Determining MKVToolNix Version"), tr("MKVMerge crashed while attempting to determine MKVToolNix version."), QMessageBox::Ok);
    }
    else if (kiExitCode != 0)
    {
        QMessageBox::critical(m_pmwMainWindow, tr("Error Determining MKVToolNix Version"), tr("An error occured while attempting to determine MKVToolNix version."), QMessageBox::Ok);
    }
    else
    {
        QRegularExpression qreVersion("mkvmerge\\s+v(\\d+)\\.(\\d+)");
        QRegularExpressionMatch qremVersion = qreVersion.match(m_qstrVersionString);
        if (qremVersion.hasMatch())
        {
            int iMajorVersion = qremVersion.captured(1).toInt();
            int iMinorVersion = qremVersion.captured(2).toInt();
            m_pimkvMKVToolNix->SetMKVToolNixVersion(iMajorVersion, iMinorVersion);
        }
        else
        {
            QMessageBox::critical(m_pmwMainWindow,
                                  tr("Error Determining MKVToolNix Version"),
                                  tr("Could not determine MKVToolNix version from:\n\n%1").arg(m_qstrVersionString.trimmed()),
                                  QMessageBox::Ok);
        }
    }
    m_qstrVersionString.clear();
}


void IComSysMKVToolNixVer::MKVMergeErrorText()
{
    QByteArray qbaData = m_qprocMKVMerge.readAllStandardError();
    QString qstrErrorText = QString::fromUtf8(qbaData);

    QMessageBox::critical(m_pmwMainWindow, tr("Error Determining MKVToolNix Version"), tr("An error occured while attempting to determine MKVToolNix version:\n\n%1").arg(qstrErrorText), QMessageBox::Ok);
}


void IComSysMKVToolNixVer::MKVMergeError(const QProcess::ProcessError kqpeError)
{
    if (kqpeError == QProcess::FailedToStart)
    {
        QMessageBox::critical(m_pmwMainWindow,
                              tr("Error Determining MKVToolNix Version"),
                              tr("An error occured while attempting to determine MKVToolNix version.\nPlease ensure the MKVToolNix directory has been correctly specified"),
                              QMessageBox::Ok);
    }
}
