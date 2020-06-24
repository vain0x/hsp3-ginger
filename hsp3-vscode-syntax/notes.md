# Notes

## Language Configuration

`language-configuration.json`

- [Language Configuration Guide | Visual Studio Code Extension API](https://code.visualstudio.com/api/language-extensions/language-configuration-guide)

### WordPattern

```js
    // 16進数
    /(0[xX]|\$)[0-9A-Fa-f]{0,8}/

    // 2進数
    /(0[bB]|%)[01]{0,99}/

    // 10進数
    /-?\d{1,20}(\.\d{0,20})?([eE][-+]?\d{0,9})?/

    // 識別子
    /[^-\/\[\](){}.?*+|^$\\\s'"~!#%&=;:,<>]{1,99}/
```
