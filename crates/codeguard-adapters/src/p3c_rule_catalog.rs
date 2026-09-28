//! P3C PMD 2.1.1 规则身份目录；从该制品内十份 ali-*.xml 规则集提取名称。
//! 本地验收制品 SHA-256: e7afec9340a0f30f56f4fdcd3b9c49e79ed24253719fcb9f457f5bcb43e54860。
//! 此摘要用于目录来源复核，不构成受保护的工具批准。

/// 核对原生 PMD 诊断的规则集及规则 ID 均属于本轮明确选择的 P3C 规则集。
/// 参数为受控规则路径、报告中的 ruleset 名称和 rule 名称；返回值只证明目录归属，
/// 不证明工具身份、规则执行覆盖或项目质量通过。
#[must_use]
pub fn p3c_rule_in_selected_rulesets(selected: &[&str], ruleset: &str, rule: &str) -> bool {
    let (path, rules): (&str, &[&str]) = match ruleset {
        "AlibabaJavaComments" => (
            "rulesets/java/ali-comment.xml",
            &[
                "CommentsMustBeJavadocFormatRule",
                "AbstractMethodOrInterfaceMethodMustUseJavadocRule",
                "ClassMustHaveAuthorRule",
                "EnumConstantsMustHaveCommentRule",
                "AvoidCommentBehindStatementRule",
                "RemoveCommentedCodeRule",
            ],
        ),
        "AlibabaJavaConcurrent" => (
            "rulesets/java/ali-concurrent.xml",
            &[
                "ThreadPoolCreationRule",
                "AvoidUseTimerRule",
                "AvoidManuallyCreateThreadRule",
                "ThreadShouldSetNameRule",
                "AvoidCallStaticSimpleDateFormatRule",
                "ThreadLocalShouldRemoveRule",
                "AvoidConcurrentCompetitionRandomRule",
                "CountDownShouldInFinallyRule",
                "LockShouldWithTryFinallyRule",
            ],
        ),
        "AlibabaJavaConstants" => (
            "rulesets/java/ali-constant.xml",
            &["UpperEllRule", "UndefineMagicConstantRule"],
        ),
        "AlibabaJavaExceptions" => (
            "rulesets/java/ali-exception.xml",
            &[
                "MethodReturnWrapperTypeRule",
                "AvoidReturnInFinallyRule",
                "TransactionMustHaveRollbackRule",
            ],
        ),
        "AlibabaJavaFlowControl" => (
            "rulesets/java/ali-flowcontrol.xml",
            &[
                "SwitchStatementRule",
                "NeedBraceRule",
                "AvoidComplexConditionRule",
                "AvoidNegationOperatorRule",
            ],
        ),
        "AlibabaJavaNaming" => (
            "rulesets/java/ali-naming.xml",
            &[
                "ClassNamingShouldBeCamelRule",
                "AbstractClassShouldStartWithAbstractNamingRule",
                "ExceptionClassShouldEndWithExceptionRule",
                "TestClassShouldEndWithTestNamingRule",
                "LowerCamelCaseVariableNamingRule",
                "AvoidStartWithDollarAndUnderLineNamingRule",
                "ConstantFieldShouldBeUpperCaseRule",
                "ServiceOrDaoClassShouldEndWithImplRule",
                "PackageNamingRule",
                "BooleanPropertyShouldNotStartWithIsRule",
                "ArrayNamingShouldHaveBracketRule",
            ],
        ),
        "AlibabaJavaOop" => (
            "rulesets/java/ali-oop.xml",
            &[
                "EqualsAvoidNullRule",
                "WrapperTypeEqualityRule",
                "PojoMustUsePrimitiveFieldRule",
                "PojoNoDefaultValueRule",
                "PojoMustOverrideToStringRule",
                "StringConcatRule",
                "BigDecimalAvoidDoubleConstructorRule",
            ],
        ),
        "AlibabaJavaOrm" => (
            "rulesets/java/ali-orm.xml",
            &["IbatisMethodQueryForListRule"],
        ),
        "AlibabaJavaOthers" => (
            "rulesets/java/ali-other.xml",
            &[
                "AvoidPatternCompileInMethodRule",
                "AvoidApacheBeanUtilsCopyRule",
                "AvoidNewDateGetTimeRule",
                "AvoidMissUseOfMathRandomRule",
                "MethodTooLongRule",
                "UseRightCaseForDateFormatRule",
                "AvoidDoubleOrFloatEqualCompareRule",
            ],
        ),
        "AlibabaJavaSets" => (
            "rulesets/java/ali-set.xml",
            &[
                "ClassCastExceptionWithToArrayRule",
                "UnsupportedExceptionWithModifyAsListRule",
                "ClassCastExceptionWithSubListToArrayListRule",
                "ConcurrentExceptionWithModifyOriginSubListRule",
                "DontModifyInForeachCircleRule",
                "CollectionInitShouldAssignCapacityRule",
            ],
        ),
        _ => return false,
    };
    selected.contains(&path) && rules.contains(&rule)
}
