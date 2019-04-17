Config = require './config'

module.exports =

  run: ->
    path = @safeBroker()
    return unless path?

    if Config.get.UsekillQuiotations()
      command = (Config.get.compilerPath() + " " + Config.get.runCommand().join(' ').replace(/\""/g, ""))
      console.log 'UsekillQuiotations',command if atom.inDevMode()
      @exec command
    else
      @execFile Config.get.compilerPath(),Config.get.runCommand()

  make: ->
    path = @safeBroker()
    return unless path?

    if Config.get.UsekillQuiotations()
      command = (Config.get.compilerPath() + " " + Config.get.makeCommand().join(' ').replace(/\""/g, ""))
      console.log 'UsekillQuiotations',command if atom.inDevMode()
      @exec command
    else
      @execFile Config.get.compilerPath(),Config.get.makeCommand()

  safeBroker: ->
    editor = atom.workspace.getActiveTextEditor()
    return unless editor?
    return unless editor.isEmpty()?

    # 未保存なら通知する。
    if editor.isModified()
      atom.notifications.addInfo(
        "Not been saved file. (language-hsp3)",
        {detail: "Please save to reflect changes in buffer."}
      )

    # editorがhspファイルを開いているか？
    return unless editor.getRootScopeDescriptor().scopes[0] is 'source.hsp3'

    return editor.getPath()

  execFile: (filePath,commands) ->
    require('child_process').execFile(
      filePath,
      commands,
      {
        encoding: Config.get.compilerEncoding()
        maxBuffer: Config.get.maxLogBuffer()
      },
      (err, stdout, stderr) ->
        option =
          detail: require('./submodel').convertToUTF8(stdout)
        if err
          option.dismissable = true
          atom.notifications.addError "Error (language-hsp3)", option
        else
          option.dismissable = Config.get.keepShowSuccessMessage()
          atom.notifications.addSuccess "Success (language-hsp3)", option
        return
    )
    return

  exec: (command) ->
    require('child_process').exec(
      command,
      {
        encoding: Config.get.compilerEncoding()
        maxBuffer: Config.get.maxLogBuffer()
      },
      (err, stdout, stderr) ->
        option =
          detail: require('./submodel').convertToUTF8(stdout)
        if err
          option.dismissable = true
          atom.notifications.addError "Error (language-hsp3)", option
        else
          option.dismissable = Config.get.keepShowSuccessMessage()
          atom.notifications.addSuccess "Success (language-hsp3)", option
        return
    )
    return

  test: ->
    console.log Config.get.replace(['%PROJECT%']) if atom.inDevMode()
    return
