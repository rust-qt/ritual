# For a complete reference, please see the online documentation at
# https://docs.vagrantup.com.

require 'yaml'
dir = File.dirname(File.expand_path(__FILE__))
vagrant_dir = "#{dir}/scripts/vagrant"

# defaults
local_settings = YAML::load_file("#{vagrant_dir}/local_settings.yml.example")

if File.exist?("#{vagrant_dir}/local_settings.yml")
    local_settings.merge!(YAML::load_file("#{vagrant_dir}/local_settings.yml"))
end

Vagrant.configure("2") do |config|

    config.vm.define "osx" do |osx|
        osx.vm.box = "jhcook/osx-yosemite-10.10"
        osx.vm.synced_folder ".", "/vagrant", type: "rsync",
            rsync__exclude: [".git/", "target/"],
            group: "admin"

        osx.vm.provision "install_dependencies", type: "shell",
            path: "scripts/vagrant/osx/install_dependencies.sh"
    end

    config.vm.define "linux" do |linux|
        linux.vm.box = "ubuntu/bionic64"
        linux.vm.provision "install_dependencies", type: "shell",
            path: "scripts/vagrant/linux/install_dependencies.sh", privileged: false
    end

    config.vm.define "windows" do |linux|
        config.vm.box = "datacastle/windows7"
        config.vm.box_version = "1.0"
    end
end
