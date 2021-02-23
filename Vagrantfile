Vagrant.configure(2) do |config|
  config.vm.box = "bento/freebsd-12.2"
  config.vm.hostname = "build"

  # Expose the nomad api and ui to the host
  # config.vm.network "forwarded_port", guest: 2222, host: 2222, auto_correct: true

  # TODO: proc count

  # Increase memory for Virtualbox
  config.vm.provider "virtualbox" do |vb|
        vb.memory = "8192"
  end

  # Install git & rust while we're at it
  config.vm.provision "shell", inline: "sudo pkg install --yes git-lite rsync fish bash openssl sqlite3 gmake protobuf"

end