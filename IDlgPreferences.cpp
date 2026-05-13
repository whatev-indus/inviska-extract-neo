#include <QtWidgets>
#include "IDlgPreferences.h"
#include "IUIMainWindow.h"
#include "IUIExtract.h"
#include "IComSysMKVToolNix.h"
#include "IComUIPrefGeneral.h"
#include "IComUtilityFuncs.h"

int IDlgPreferences::m_iStartPage = 0;


IDlgPreferences::IDlgPreferences(IUIMainWindow* pmwMainWindow, const int kiStartPage) : QDialog(pmwMainWindow),
                                                                                        m_rqsetSettings(pmwMainWindow->GetSettings())
{
    m_pmwMainWindow = pmwMainWindow;

    setupUi(this);
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowFlags(this->windowFlags() & ~Qt::WindowContextHelpButtonHint);
    setModal(true);

    m_pipgGeneralPreferences = new IComUIPrefGeneral(this, pmwMainWindow);
    m_pqswPageStack->insertWidget(0, m_pipgGeneralPreferences);

    QWidget* pqwdgThemesPage = new QWidget(this);
    QLabel* pqlblTheme = new QLabel(tr("Theme:"), pqwdgThemesPage);
    m_pqlwTheme = new QListWidget(pqwdgThemesPage);
    m_pqlwTheme->setSelectionMode(QAbstractItemView::SingleSelection);
    m_pqlwTheme->setMaximumWidth(180);
    m_pqlwTheme->setMaximumHeight(96);

    QListWidgetItem* pqlwiNeoTheme = new QListWidgetItem(tr("Neo"), m_pqlwTheme);
    pqlwiNeoTheme->setData(Qt::UserRole, "Neo");
    QListWidgetItem* pqlwiDarkTheme = new QListWidgetItem(tr("Dark"), m_pqlwTheme);
    pqlwiDarkTheme->setData(Qt::UserRole, "Dark");
    QListWidgetItem* pqlwiLightTheme = new QListWidgetItem(tr("Light"), m_pqlwTheme);
    pqlwiLightTheme->setData(Qt::UserRole, "Light");

    QVBoxLayout* pqvblThemesLayout = new QVBoxLayout;
    pqvblThemesLayout->addWidget(pqlblTheme);
    pqvblThemesLayout->addWidget(m_pqlwTheme);
    pqvblThemesLayout->addStretch();
    pqwdgThemesPage->setLayout(pqvblThemesLayout);
    m_pqlwPageList->addItem(tr("Themes"));
    m_pqswPageStack->addWidget(pqwdgThemesPage);

    SetWidgetStates();
    ApplyTheme(m_pqlwTheme->currentItem()->data(Qt::UserRole).toString());
    connect(m_pqlwTheme, &QListWidget::currentItemChanged, this, [this](QListWidgetItem* pqlwiCurrent) {
        if (pqlwiCurrent != nullptr)
            ApplyTheme(pqlwiCurrent->data(Qt::UserRole).toString());
    });
    if (kiStartPage != -1)
        m_iStartPage = kiStartPage;
    m_pqlwPageList->setCurrentRow(m_iStartPage);
    m_pqswPageStack->setCurrentIndex(m_iStartPage);

    m_pqpbMKVToolNixLocate->setIcon(m_pmwMainWindow->style()->standardIcon(QStyle::SP_DirOpenIcon));
    connect(m_pqpbMKVToolNixLocate, &QPushButton::clicked, this, &IDlgPreferences::SetMKVToolNixPath);

    resize(sizeHint());
    show();
}


void IDlgPreferences::SetWidgetStates()
{   
    m_pipgGeneralPreferences->SetWidgetStates();

    m_qstrMKVToolNixPathOrig = QDir::toNativeSeparators(m_rqsetSettings.value("MKVToolNix/MKVToolNixPath", "").toString());
    #ifdef Q_OS_MACOS
    if (m_qstrMKVToolNixPathOrig.endsWith("/Contents/MacOS"))
        m_qstrMKVToolNixPathOrig.truncate(m_qstrMKVToolNixPathOrig.length() - 15);
    #endif
    m_pqleMKVToolNixDir->setText(m_qstrMKVToolNixPathOrig);
    m_pqleMKVToolNixDir->setCursorPosition(0);

    m_pqcbDetectCuesheetsTags->setChecked(m_pmwMainWindow->GetExtractUI()->DetectCuesheetsTags());

    const QString qstrThemeName = m_rqsetSettings.contains("Appearance/Theme") ? m_rqsetSettings.value("Appearance/Theme").toString() : m_pmwMainWindow->GetExtractUI()->GetThemeName();
    QList<QListWidgetItem*> qlstThemeItems = m_pqlwTheme->findItems("*", Qt::MatchWildcard);
    QListWidgetItem* pqlwiSelectedTheme = nullptr;
    for (QListWidgetItem* pqlwiTheme : qlstThemeItems)
    {
        if (pqlwiTheme->data(Qt::UserRole).toString() == qstrThemeName)
        {
            pqlwiSelectedTheme = pqlwiTheme;
            break;
        }
    }
    if (pqlwiSelectedTheme == nullptr)
        pqlwiSelectedTheme = m_pqlwTheme->item(0);
    m_pqlwTheme->setCurrentItem(pqlwiSelectedTheme);
}


bool IDlgPreferences::SaveSettings()
{
    if (m_pipgGeneralPreferences->SaveSettings() == false)
        return false;

    m_rqsetSettings.setValue("MKVToolNix/DetectCuesheetsTags", m_pqcbDetectCuesheetsTags->isChecked());
    m_rqsetSettings.setValue("Appearance/Theme", m_pqlwTheme->currentItem()->data(Qt::UserRole).toString());

    m_rqsetSettings.sync();
    ProcessChanges();

    return true;
}


void IDlgPreferences::ProcessChanges()
{
    m_pipgGeneralPreferences->ProcessChanges();

    if (m_bMKVToolNixPathChanged)
    {
        IComSysMKVToolNix* pimkvMKVToolNix = m_pmwMainWindow->GetExtractUI()->GetMKVToolNix();
        pimkvMKVToolNix->SetMKVToolNixPath(m_pqleMKVToolNixDir->text());
        pimkvMKVToolNix->DetermineMKVToolNixVersion();
    }

    m_pmwMainWindow->GetExtractUI()->SetDetectCuesheetsTags(m_pqcbDetectCuesheetsTags->isChecked());
    m_pmwMainWindow->GetExtractUI()->SetTheme(m_pqlwTheme->currentItem()->data(Qt::UserRole).toString());
}


void IDlgPreferences::SetMKVToolNixPath()
{
    QFileDialog qfdFileDlg(this);
    #ifdef Q_OS_MACOS
    qfdFileDlg.setFileMode(QFileDialog::ExistingFile);
    qfdFileDlg.setOptions(QFileDialog::DontResolveSymlinks);
    #else
    qfdFileDlg.setFileMode(QFileDialog::Directory);
    qfdFileDlg.setOptions(QFileDialog::ShowDirsOnly | QFileDialog::DontResolveSymlinks);
    #endif

    qfdFileDlg.setWindowTitle(tr("Specify MKVToolNix Directory"));

    #ifdef Q_OS_WIN
    qfdFileDlg.setDirectoryUrl(IComUtilityFuncs::GetMyComputerURL());
    #elif defined Q_OS_MACOS
    qfdFileDlg.setDirectory(QDir("/Applications").exists() ? "/Applications" : QDir::homePath() + "/Applications");
    #else
    qfdFileDlg.setDirectory(QDir::rootPath());
    #endif

    if (qfdFileDlg.exec())
    {
        QString qstrDirectory = qfdFileDlg.selectedFiles().at(0);
        if (ValidateMKVToolNixPath(qstrDirectory) == true)
            m_pqleMKVToolNixDir->setText(QDir::toNativeSeparators(qstrDirectory));
    }
}




void IDlgPreferences::ApplyTheme(const QString & krqstrThemeName)
{
    QString qstrWindow;
    QString qstrPanel;
    QString qstrField;
    QString qstrText;
    QString qstrMuted;
    QString qstrBorder;
    QString qstrButton;
    QString qstrButtonHover;
    QString qstrButtonText;
    QString qstrSelection;
    QString qstrSelectionText;
    QString qstrArrow;

    if (krqstrThemeName == "Light")
    {
        qstrWindow = "#f4f5f7";
        qstrPanel = "#edf0f3";
        qstrField = "#ffffff";
        qstrText = "#3f4752";
        qstrMuted = "#667085";
        qstrBorder = "#d0d5dd";
        qstrButton = "#e5e7eb";
        qstrButtonHover = "#d9dde3";
        qstrButtonText = qstrText;
        qstrSelection = "#2563eb";
        qstrSelectionText = "#ffffff";
        qstrArrow = ":/Resources/BranchClosed.svg";
    }
    else if (krqstrThemeName == "Dark")
    {
        qstrWindow = "#333333";
        qstrPanel = "#333333";
        qstrField = "#1d1d1d";
        qstrText = "#eeeeee";
        qstrMuted = "#8f8f8f";
        qstrBorder = "#4a4a4a";
        qstrButton = "#555555";
        qstrButtonHover = "#606060";
        qstrButtonText = "#ffffff";
        qstrSelection = "#555555";
        qstrSelectionText = "#ffffff";
        qstrArrow = ":/Resources/BranchClosedLight.svg";
    }
    else
    {
        qstrWindow = "#0f1117";
        qstrPanel = "#1a1d27";
        qstrField = "#111318";
        qstrText = "#e8eaf6";
        qstrMuted = "#7b82a8";
        qstrBorder = "#2e3250";
        qstrButton = "#2f6feb";
        qstrButtonHover = "#5b8af5";
        qstrButtonText = "#ffffff";
        qstrSelection = "#5b8af5";
        qstrSelectionText = "#ffffff";
        qstrArrow = ":/Resources/BranchClosedLight.svg";
    }

    setStyleSheet(QString(
        "IDlgPreferences, IDlgPreferences QWidget {"
        "  background-color: %1;"
        "  color: %4;"
        "}"
        "IDlgPreferences QLabel, IDlgPreferences QCheckBox, IDlgPreferences QRadioButton, IDlgPreferences QGroupBox {"
        "  color: %4;"
        "}"
        "IDlgPreferences QGroupBox {"
        "  background-color: %2;"
        "  border: 1px solid %6;"
        "  border-radius: 3px;"
        "  margin-top: 8px;"
        "  padding: 8px 8px 6px 8px;"
        "}"
        "IDlgPreferences QGroupBox::title {"
        "  subcontrol-origin: margin;"
        "  left: 8px;"
        "  padding: 0px 4px;"
        "}"
        "IDlgPreferences QListWidget, IDlgPreferences QLineEdit, IDlgPreferences QComboBox {"
        "  background-color: %3;"
        "  color: %4;"
        "  border: 1px solid %6;"
        "  selection-background-color: %10;"
        "  selection-color: %11;"
        "}"
        "IDlgPreferences QListWidget::item {"
        "  padding: 4px 6px;"
        "}"
        "IDlgPreferences QListWidget::item:selected {"
        "  background-color: %10;"
        "  color: %11;"
        "}"
        "IDlgPreferences QLineEdit:disabled {"
        "  background-color: %2;"
        "  color: %5;"
        "}"
        "IDlgPreferences QComboBox {"
        "  padding: 2px 22px 2px 6px;"
        "}"
        "IDlgPreferences QComboBox::drop-down {"
        "  subcontrol-origin: padding;"
        "  subcontrol-position: top right;"
        "  width: 20px;"
        "  background-color: %7;"
        "  border-left: 1px solid %6;"
        "}"
        "IDlgPreferences QComboBox::down-arrow {"
        "  image: url(%12);"
        "  width: 12px;"
        "  height: 12px;"
        "}"
        "IDlgPreferences QPushButton {"
        "  background-color: %7;"
        "  color: %9;"
        "  border: 1px solid %7;"
        "  padding: 2px 10px;"
        "}"
        "IDlgPreferences QPushButton:hover {"
        "  background-color: %8;"
        "  border-color: %8;"
        "}"
    ).arg(qstrWindow, qstrPanel, qstrField, qstrText, qstrMuted, qstrBorder,
          qstrButton, qstrButtonHover, qstrButtonText, qstrSelection,
          qstrSelectionText, qstrArrow));
}


bool IDlgPreferences::ValidateMKVToolNixPath(const QString & krqstrPath)
{
    IComSysMKVToolNix* pimkvMKVToolNix = m_pmwMainWindow->GetExtractUI()->GetMKVToolNix();
    if (pimkvMKVToolNix->ValidateMKVToolNixPath(krqstrPath) == false)
    {
        QMessageBox::warning(this, tr("MKVToolNix Location Invalid"), tr("MKVToolNix cannot be found at the specified location.\nPlease specify a valid location for MKVToolNix.\nUse Cancel to exit Preferences without specifying a directory."), QMessageBox::Ok);
        return false;
    }
    return true;
}


void IDlgPreferences::accept()
{
    m_bMKVToolNixPathChanged = (m_pqleMKVToolNixDir->text() != m_qstrMKVToolNixPathOrig);

    if (m_bMKVToolNixPathChanged && ValidateMKVToolNixPath(m_pqleMKVToolNixDir->text()) == false)
        return;

    if (SaveSettings() == false)
        return;

    QDialog::accept();
}


void IDlgPreferences::done(const int kiResult)
{
    m_iStartPage = m_pqlwPageList->currentRow();
    QDialog::done(kiResult);
}
