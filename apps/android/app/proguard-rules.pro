# Proguard rules for RBD OFIS Android
# Preserve JNI methods and native bridge bindings
-keepclasseswithmembernames class * {
    native <methods>;
}

-keep class com.rbdofis.app.core.** { *; }
