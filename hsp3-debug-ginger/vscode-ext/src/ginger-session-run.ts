import { GingerDebugSession } from './ginger-session';
import { DebugSession } from 'vscode-debugadapter';

GingerDebugSession.run(GingerDebugSession as typeof DebugSession);
