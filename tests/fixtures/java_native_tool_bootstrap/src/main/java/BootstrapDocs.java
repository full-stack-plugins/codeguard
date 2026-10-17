/**
 * 提供工具缓存准备时使用的最小文档样例，无业务或外部依赖。
 */
public final class BootstrapDocs {
    /**
     * 创建无状态的文档样例对象。
     */
    public BootstrapDocs() {
    }

    /**
     * 原样返回输入文本，不执行转换或外部调用。
     *
     * @param value 待返回的文本，允许为 null
     * @return 与输入完全相同的文本引用，输入为 null 时返回 null
     */
    public String echo(String value) {
        return value;
    }
}
