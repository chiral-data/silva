# Silva Documentation

## Installation

### Install prebuilt binaries via shell script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/chiral-data/silva/releases/download/v0.2.2/silva-tui-installer.sh | sh
```

#### The solution for missing libopenssl1.1 under Ubuntu
```
curl -O http://security.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.24_amd64
sudo dpkg -i libssl1.1_1.1.1f-1ubuntu2.24_amd64.deb            
```

### Install prebuilt binaries via powershell script

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/chiral-data/silva/releases/download/v0.2.2/silva-tui-installer.ps1 | iex"
```



## Environmental variables

### Project home folder
- SILVA_PROJECTS_HOME (option): if not set, a folder "my-silva-projects" will be created and used as home folder for projects

### Sakura Internet Access Token
- SILVA_SAKURA_RESOURCE_ID
- SILVA_SAKURA_ACCESS_TOKEN
- SILVA_SAKURA_ACCESS_TOKEN_SECRET
The information can be found from the page "API Key" from Sakura Internet Website.

```sh
# set under MacOS/Linux
export SILVA_SAKURA_ACCESS_TOKEN=""
export SILVA_SAKURA_ACCESS_TOKEN_SECRET=""
export SILVA_SAKURA_RESOURCE_ID=""
```

```sh
[Environment]::SetEnvironmentVariable('SILVA_SAKURA_ACCESS_TOKEN','')
[Environment]::SetEnvironmentVariable('SILVA_SAKURA_ACCESS_TOKEN_SECRET','')
[Environment]::SetEnvironmentVariable('SILVA_SAKURA_RESOURCE_ID','')
```



### Quickstart
#### Copy tutorial project
TBD





## Examples
- [Ollama](./apps/Ollama.md)
- Gromacs: TBD
- Whisper: TBD
