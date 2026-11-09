# Proguard rules for RBD OFIS Android
# Preserve JNI methods and native bridge bindings
-keepclasseswithmembernames class * {
    native <methods>;
}

-keep class com.rbdofis.app.core.** { *; }

# Preserve data repositories, cache structures, and data models
-keep class com.rbdofis.app.data.** { *; }

# Preserve data model classes, states, and enums
-keep class com.rbdofis.app.AppScreen { *; }
-keep enum com.rbdofis.app.** { *; }
-keep class com.rbdofis.app.ui.AiSummaryState { *; }
-keep class com.rbdofis.app.ui.AiAnswerState { *; }
-keep class com.rbdofis.app.ui.viewer.ViewerUiState { *; }
-keep class com.rbdofis.app.ui.organizer.OrganizerUiState { *; }
-keep class com.rbdofis.app.ui.scanner.DocumentScannerState { *; }
-keepclassmembers class * implements java.io.Serializable {
    static final long serialVersionUID;
    private static final java.io.ObjectStreamField[] serialPersistentFields;
    !static !transient <fields>;
    private void writeObject(java.io.ObjectOutputStream);
    private void readObject(java.io.ObjectInputStream);
    java.lang.Object writeReplace();
    java.lang.Object readResolve();
}

