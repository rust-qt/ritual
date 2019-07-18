#!/usr/bin/env python3

from bs4 import BeautifulSoup
from urllib.request import urlopen
import shutil
import subprocess
import os

versions = ['5.9.7', '5.11.3', '5.12.2', '5.13.0']

def get_links(url):
    print('Downloading file list from {}'.format(url))
    html_page = urlopen(remote_dir).read().decode('utf-8')
    soup = BeautifulSoup(html_page, features="lxml")
    links = [
        link.get('href')
        for link in soup.findAll('a')
        if link.get('href').endswith('.7z')
    ]
    links.sort()
    return links

def install(url, message_caption):
    tmp_path = '/tmp/1.7z'
    print('Downloading {} from {}'.format(message_caption, url))
    with urlopen(file_url) as response, open(tmp_path, 'wb') as file:
        shutil.copyfileobj(response, file)
    subprocess.run([
        "7z",
        "x", # extract
        tmp_path
    ], check=True)

for version in versions:
    print('Installing Qt {}'.format(version))
    version_uglified = version.replace('.', '')
    remote_dir = (
        'http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/'
        'qt5_{0}/qt.qt5.{0}.gcc_64/'.format(version_uglified)
    )
    links = get_links(remote_dir)

    for module in ['icu', 'qtbase', 'qttools', 'qtgamepad', 'qt3d']:
        file_names = list(filter(lambda x: module in x, links))
        if not file_names:
            raise Exception("Can't find package {} for {}".format(module, version))
        latest_file_name = file_names[-1]
        file_url = remote_dir + latest_file_name
        install(file_url, module)

for version in versions:
    print('Installing docs for Qt {}'.format(version))
    version_uglified = version.replace('.', '')
    remote_dir = (
        'http://download.qt.io/online/qtsdkrepository/linux_x64/desktop/'
        'qt5_{0}_src_doc_examples/qt.qt5.{0}.doc/'.format(version_uglified)
    )
    links = get_links(remote_dir)

    if any('qt-everywhere-documentation' in x for x in links):
        modules = ['qt-everywhere-documentation']
    else:
        modules = ['qtcore', 'qtgui', 'qtwidgets', 'qtuitools', 'qt3d']
    for module in modules:
        file_names = list(filter(lambda x: module in x, links))
        if not file_names:
            raise Exception("Can't find package {} for {}".format(module, version))
        latest_file_name = file_names[-1]
        file_url = remote_dir + latest_file_name
        install(file_url, module)

current_dir = os.getcwd()
for name in os.listdir(current_dir):
    if name == 'Docs': continue
    parent_dir = os.path.join(current_dir, name)
    for name2 in os.listdir(parent_dir):
        dir = os.path.join(parent_dir, name2)
        print('Configuring Qt installation in {}'.format(dir))
        config_file = os.path.join(dir, 'bin', 'qt.conf')
        with open(config_file, 'w') as file:
            file.write("[Paths]\nPrefix = ..\nDocumentation=../../Docs/Qt-{}".format(name))
