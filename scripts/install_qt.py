#!/usr/bin/env python3

from bs4 import BeautifulSoup
from urllib.request import urlopen
import shutil
import subprocess
import os
import sys
import tempfile

def get_links(url):
    print('Downloading file list from {}'.format(url))
    html_page = urlopen(url).read().decode('utf-8')
    soup = BeautifulSoup(html_page, features="lxml")
    links = [
        link.get('href')
        for link in soup.findAll('a')
        if link.get('href').endswith('.7z')
    ]
    links.sort()
    return links

def install(file_url, message_caption):
    tmp_dir = tempfile.gettempdir()
    tmp_path = os.path.join(tmp_dir, '1.7z')
    print('Downloading {} from {}'.format(message_caption, file_url))
    with urlopen(file_url) as response, open(tmp_path, 'wb') as file:
        shutil.copyfileobj(response, file)
    subprocess.run([
        '7z',
        'x', # extract
        tmp_path
    ], check=True)

def link_matches(link, text):
    return ((text + '-') in link) or ((text + '.') in link)

def install_dir(dir, modules):
    remote_dir = (
        'http://download.qt.io/online/qtsdkrepository/{}/desktop/{}'
        .format(the_os, dir)
    )
    links = get_links(remote_dir)

    for module in modules:
        file_names = list(filter(lambda x: link_matches(x, module), links))
        if not file_names:
            print('links: ', links)
            print('dir', dir)
            raise Exception("Can't find package {} for {}".format(module, version))
        latest_file_name = file_names[-1]
        file_url = remote_dir + latest_file_name
        install(file_url, module)

version = sys.argv[1]
minor_version = int(version.split('.')[1])

if sys.argv[2] == '--docs':
    the_os = 'linux_x64' # any existing OS will suffice
    print('Installing docs for Qt {}'.format(version))
    version_uglified = version.replace('.', '')
    if minor_version >= 13:
        modules = [
            'qtcore', 'qtgui', 'qtwidgets', 'qtuitools', 'qtgamepad', 'qt3d',
            'qtquickcontrols', 'qtquickcontrols1', 'qtmultimedia', 'qtwebview',
            'qtwebsockets', 'qtwebchannel', 'qtsvg', 'qtspeech', 'qtserialport', 'qtserialbus',
            'qtscxml', 'qtremoteobjects', 'qtlocation', 'qtimageformats',
            'qtgraphicaleffects', 'qtqml'
        ]
    else:
        modules = ['qt-everywhere-documentation']
    install_dir(
        'qt5_{0}_src_doc_examples/qt.qt5.{0}.doc/'.format(version_uglified),
        modules
    )
    install_dir(
        'qt5_{0}_src_doc_examples/qt.qt5.{0}.doc.qtwebengine/'.format(version_uglified),
        ['qtwebengine']
    )
    install_dir(
        'qt5_{0}_src_doc_examples/qt.qt5.{0}.doc.qtcharts/'.format(version_uglified),
        ['qtcharts']
    )
    install_dir(
        'qt5_{0}_src_doc_examples/qt.qt5.{0}.doc.qtdatavis3d/'.format(version_uglified),
        ['qtdatavisualization']
    )
    sys.exit(0)

the_os = sys.argv[2]
compiler = sys.argv[3]

print('Installing Qt {}'.format(version))
version_uglified = version.replace('.', '')
modules = [
    'qtbase', 'qttools', 'qtgamepad', 'qt3d', 'qtquickcontrols',
    'qtquickcontrols2', 'qtmultimedia', 'qtwebview', 'qtwebsockets',
    'qtwebchannel', 'qtsvg', 'qtserialport', 'qtserialbus',
    'qtscxml', 'qtlocation', 'qtimageformats',
    'qtgraphicaleffects', 'qtdeclarative'
]
if the_os == 'linux_x64':
    modules.extend(['icu'])
if the_os == 'windows_x86':
    modules.extend(['opengl32sw'])
if minor_version > 9:
    modules.extend(['qtspeech'])
if minor_version > 11:
    modules.extend(['qtremoteobjects'])
install_dir(
    'qt5_{0}/qt.qt5.{0}.{1}/'.format(version_uglified, compiler),
    modules
)
install_dir(
    'qt5_{0}/qt.qt5.{0}.qtwebengine.{1}/'.format(version_uglified, compiler),
    ['qtwebengine']
)
install_dir(
    'qt5_{0}/qt.qt5.{0}.qtcharts.{1}/'.format(version_uglified, compiler),
    ['qtcharts']
)
install_dir(
    'qt5_{0}/qt.qt5.{0}.qtdatavis3d.{1}/'.format(version_uglified, compiler),
    ['qtdatavis3d']
)

current_dir = os.getcwd()

for name in os.listdir(current_dir):
    if name == 'Docs': continue
    parent_dir = os.path.join(current_dir, name)
    for name2 in os.listdir(parent_dir):
        dir = os.path.join(parent_dir, name2)
        config_file = os.path.join(dir, 'bin', 'qt.conf')
        if not os.path.isfile(config_file):
            print('Configuring Qt installation in {}'.format(dir))
            with open(config_file, 'w') as file:
                file.write("[Paths]\nPrefix=..\nDocumentation=../../Docs/Qt-{}".format(name))
