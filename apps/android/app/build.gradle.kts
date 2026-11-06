plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.rbdofis.app"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.rbdofis.app"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }

        ndk {
            // Target modern 64-bit and 32-bit ARM/x86 architectures
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86"))
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            signingConfig = signingConfigs.getByName("debug")
        }
        debug {
            applicationIdSuffix = ".debug"
            isDebuggable = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
        freeCompilerArgs = freeCompilerArgs + listOf(
            "-opt-in=androidx.compose.material3.ExperimentalMaterial3Api",
            "-opt-in=kotlinx.coroutines.ExperimentalCoroutinesApi"
        )
    }

    buildFeatures {
        compose = true
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

dependencies {
    // AndroidX Core & Lifecycle
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.activity.compose)

    // Jetpack Compose & Material 3
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.material.icons.extended)

    // CameraX (Document Scanner)
    implementation(libs.androidx.camera.core)
    implementation(libs.androidx.camera.camera2)
    implementation(libs.androidx.camera.lifecycle)
    implementation(libs.androidx.camera.view)

    // Coroutines
    implementation(libs.kotlinx.coroutines.core)
    implementation(libs.kotlinx.coroutines.android)

    // Unit Testing
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)

    // Debugging Tooling
    debugImplementation(libs.androidx.compose.ui.tooling)
}

tasks.withType<Test> {
    jvmArgs("-Dfile.encoding=UTF-8", "-Duser.language=en", "-Duser.country=US")
    testLogging {
        events("passed", "skipped", "failed", "standardOut", "standardError")
    }
}

tasks.register<JavaExec>("runUnitTests") {
    group = "verification"
    description = "Runs all unit tests with direct classpath."
    dependsOn("compileDebugKotlin", "compileDebugUnitTestKotlin")

    val testClassesDir = layout.buildDirectory.dir("tmp/kotlin-classes/debugUnitTest")
    val appClassesDir = layout.buildDirectory.dir("tmp/kotlin-classes/debug")

    classpath = files(
        testClassesDir,
        appClassesDir,
        configurations.matching { it.name == "debugUnitTestRuntimeClasspath" }
    )
    mainClass.set("org.junit.runner.JUnitCore")
    args(
        "com.rbdofis.app.LocalizationCompletenessTest",
        "com.rbdofis.app.RbdNativeBridgeTest",
        "com.rbdofis.app.PdfDocumentRepositoryTest",
        "com.rbdofis.app.InkAnnotationTest",
        "com.rbdofis.app.DocumentScannerTest",
        "com.rbdofis.app.ProToolsTest",
        "com.rbdofis.app.DocumentConverterTest",
        "com.rbdofis.app.OfficeDocumentTest",
        "com.rbdofis.app.DocumentAiTest"
    )
    jvmArgs("-Dfile.encoding=UTF-8", "-Duser.language=en", "-Duser.country=US")
}

