
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient"

import { TextDocument, CancellationToken, } from "vscode"

import { SemanticTokensFeature, DocumentSemanticsTokensSignature } from 'vscode-languageclient/lib/semanticTokens.proposed';

// Workaround for https://github.com/microsoft/vscode-languageserver-node/issues/576
export const provideDocumentSemanticTokens = async (document: TextDocument, token: CancellationToken, next: DocumentSemanticsTokensSignature) => {
  const res = await next(document, token);
  if (res === undefined) throw new Error('busy');
  return res;
}
