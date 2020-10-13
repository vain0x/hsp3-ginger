module.exports =
  convertToUTF8: (binary) ->
    iconv = require('iconv-lite')
    Config = require('./config')
    iconv.decode(new Buffer(binary,'binary'),Config.get.compilerEncoding())

  getEditFilepath: ->
    editor = atom.workspace.getActiveTextEditor()
    return unless editor?
    return unless editor.isEmpty()?
    editor.getPath()

  getProjectRoot: ->
    for elm,i in atom.project.getPaths()
      return elm if @getEditFilepath().search(elm.replace(/\\/g,'\\\\')) is 0
    return '' # 見つけられなかったら、空文字を返す。

  isProjectFilepath: ->
    editor = @getSafeEditor()
    return unless editor?
    path = editor.getPath()
    return unless path?
    atom.project.contains(path)
