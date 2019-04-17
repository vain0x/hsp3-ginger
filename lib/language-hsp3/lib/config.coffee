module.exports =
  config:

    compiler:
      order: 1
      title: 'Compiler Settings'
      type: 'object'
      properties:

        path:
          order: 1
          title: 'Compiler path'
          description: '使用するコンパイラを絶対パスで指定してください。'
          type: 'string'
          default: 'C:/hsp35/hspc.exe'

        encoding:
          order: 2
          title: 'Compiler output encoding'
          description: 'コンパイラの返信をエンコードして受け取ります。'
          type: 'string'
          default: 'Shift_JIS'

        maxLogBuffer:
          order: 3
          description: 'コンパイラの返信受取バッファサイズ(byte)\n
                        バッファサイズが足りなければ、返信の受け取りは失敗します。'
          type: "integer"
          default: 204800
          minimum: 204800
          maximum: null

        keepShowSuccessMessage:
          order: 4
          description: "コンパイルに成功した場合、メッセージを表示し続ける。"
          type: "boolean"
          default: true

    commands:
      order: 2
      title: 'Compile Command Settings'
      type: 'object'
      properties:
        runCommand:
          order: 1
          description: 'デバッグ実行で使用するパラメータ。
                        `,`で区切ることで、複数のパラメータを渡せます。
                        特殊文字で以下の文字に置き換わります。<br/>
                        `%FILEPATH%` : 現在エディタで開いているパス<br/>
                        `%PROJECT%` : 現在のProjectのルートディレクトリ'
          type: 'array'
          default: ['-Crdw','%FILEPATH%']

        makeCommand:
          order: 2
          description: '自動実行ファイル作成で使用するパラメータ。'
          type: 'array'
          default: ['-CPm','%FILEPATH%']

    extensionOptions:
      order: 3
      title: 'Extension Option Settings'
      type: 'object'
      properties:
        UsekillQuiotations:
          order: 1
          title: 'Delete quotation character'
          description: 'ソースファイルパスにダブルクオーテーションを付けない。'
          type: 'boolean'
          default: true

  get:
    replace: (arr) ->
      return unless arr?

      subModel = require './submodel'
      result = new Array()
      for element,i in arr
        result.push element.replace(
          /%FILEPATH%|%PROJECT%/g,
          (match) ->
            return subModel.getEditFilepath() if match is '%FILEPATH%'
            return subModel.getProjectRoot() if match is '%PROJECT%'
            match
        )
      result

    compilerPath: ->
      atom.config.get('language-hsp3.compiler').path

    compilerEncoding: ->
      atom.config.get('language-hsp3.compiler').encoding

    maxLogBuffer: ->
      atom.config.get('language-hsp3.compiler').maxLogBuffer

    keepShowSuccessMessage: ->
      atom.config.get('language-hsp3.compiler').keepShowSuccessMessage

    runCommand: ->
      @replace(atom.config.get('language-hsp3.commands').runCommand)

    makeCommand: ->
      @replace(atom.config.get('language-hsp3.commands').makeCommand)

    UsekillQuiotations: ->
      atom.config.get('language-hsp3.extensionOptions').UsekillQuiotations
