Model = require './model'
Config = require './config'

module.exports =

  config: Config.config

  activate: (state) ->
    atom.commands.add 'atom-workspace','language-hsp3:run': -> Model.run()
    atom.commands.add 'atom-workspace','language-hsp3:make': -> Model.make()
    return
