QT += core gui widgets network
TEMPLATE = app

VERSION = 1.1.0
DISPLAY_VERSION = v1.1.0
NAME = "Inviska Extract Neo"
NAME_NO_SPACES = "InviskaExtractNeo"

HEADERS += \
    IComSysAbsoluteDay.h \
    IComSysLatestVersion.h \
    IComSysMKVToolNix.h \
    IComSysMKVToolNixVer.h \
    IComSysSingleInstance.h \
    IComUIMainWinBase.h \
    IComUIMenuBarBase.h \
    IComUIPrefGeneral.h \
    IComUtilityFuncs.h \
    IDlgExtractProgress.h \
    IDlgPreferences.h \
    IMKVAttachmentInfo.h \
    IMKVCodecLookup.h \
    IMKVExtractProcess.h \
    IMKVFileInfo.h \
    IMKVTrackInfo.h \
    IUIExtract.h \
    IUIMainWindow.h \
    IUIMenuBar.h \
    IMacMenu.h \
    IMKVChaptCueshtTagInfo.h

SOURCES += \
    IComSysAbsoluteDay.cpp \
    IComSysLatestVersion.cpp \
    IComSysMKVToolNix.cpp \
    IComSysMKVToolNixVer.cpp \
    IComSysSingleInstance.cpp \
    IComUIMainWinBase.cpp \
    IComUIMenuBarBase.cpp \
    IComUIPrefGeneral.cpp \
    IComUtilityFuncs.cpp \
    InviskaMain.cpp \
    IDlgExtractProgress.cpp \
    IDlgPreferences.cpp \
    IMKVAttachmentInfo.cpp \
    IMKVCodecLookup.cpp \
    IMKVExtractProcess.cpp \
    IMKVFileInfo.cpp \
    IMKVTrackInfo.cpp \
    IUIExtract.cpp \
    IUIMainWindow.cpp \
    IUIMenuBar.cpp \
    IMKVChaptCueshtTagInfo.cpp

FORMS += \
    UIComPrefGeneral.ui \
    UIPreferences.ui

RESOURCES += \
    Resources.qrc

DEFINES += \
    QT_DEPRECATED_WARNINGS \
    APP_VERSION=\"\\\"$${DISPLAY_VERSION}\\\"\" \
    APP_NAME=\"\\\"$${NAME}\\\"\" \
    APP_NAME_NO_SPACES=\"\\\"$${NAME_NO_SPACES}\\\"\"

CONFIG(release, debug|release):DEFINES += QT_NO_DEBUG_OUTPUT


win32 {
    TARGET = $$NAME_NO_SPACES
    QMAKE_TARGET_PRODUCT = $$NAME
    QMAKE_TARGET_DESCRIPTION = $$NAME
    QMAKE_TARGET_COMPANY = "Inviska Software"
    QMAKE_TARGET_COPYRIGHT = "Released under the GPLv2 licence"
    RC_LANG = 0x0809
    RC_CODEPAGE = 1252
    RC_ICONS = ./Resources/Icon.ico
    QMAKE_LFLAGS *= -static-libgcc -static-libstdc++ -static -lpthread
    lessThan(QT_MAJOR_VERSION, 6):QT += winextras
}


macx {
    TARGET = $$NAME
    ICON = ./Resources/Icon.icns
    QMAKE_INFO_PLIST = ./Resources/Info.plist
    QMAKE_CFLAGS_RELEASE *= -fvisibility=hidden
    QMAKE_CXXFLAGS_RELEASE *= -fvisibility=hidden -fvisibility-inlines-hidden
    OBJECTIVE_SOURCES += IMacMenu.mm
}


unix:!macx {
    TARGET = invmextr

    QMAKE_CPPFLAGS *= -Wdate-time -D_FORTIFY_SOURCE=2
    QMAKE_CFLAGS   *= -fPIE -g -O2 -fdebug-prefix-map=/home/user=. -fstack-protector-strong -Wformat -Werror=format-security
    QMAKE_CXXFLAGS *= -fPIE -g -O2 -fdebug-prefix-map=/home/user=. -fstack-protector-strong -Wformat -Werror=format-security
    QMAKE_LFLAGS   *= -pie -Wl,-Bsymbolic-functions -Wl,-z,relro,-z,now
}


# For building release from command line - qmake CONFIG+=BuildRelease
contains(CONFIG, BuildRelease) {
    message("Building the RELEASE Version")
    CONFIG -= debug_and_release
    CONFIG -= debug
    CONFIG += release

    OUTPUTDIR = release
    DESTDIR = $$OUTPUTDIR
    OBJECTS_DIR = $$OUTPUTDIR
    MOC_DIR = $$OUTPUTDIR
    RCC_DIR = $$OUTPUTDIR
    UI_DIR = $$OUTPUTDIR
}


# For building debug from command line - qmake CONFIG+=BuildDebug
contains(CONFIG, BuildDebug) {
    message("Building the DEBUG Version")
    CONFIG -= debug_and_release
    CONFIG -= release
    CONFIG += debug

    OUTPUTDIR = debug
    DESTDIR = $$OUTPUTDIR
    OBJECTS_DIR = $$OUTPUTDIR
    MOC_DIR = $$OUTPUTDIR
    RCC_DIR = $$OUTPUTDIR
    UI_DIR = $$OUTPUTDIR
}
