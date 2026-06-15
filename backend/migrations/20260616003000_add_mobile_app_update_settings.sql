INSERT INTO system_settings (key, value, description)
VALUES
    ('mobile_app_android_enabled', 'false', 'Android APP 更新检查开关'),
    ('mobile_app_android_latest_version', '0.1.0', 'Android APP 最新版本号'),
    ('mobile_app_android_latest_build', '1', 'Android APP 最新构建号，数字越大版本越新'),
    ('mobile_app_android_package_url', '未配置', 'Android APK 安装包下载链接'),
    ('mobile_app_android_force_update', 'false', 'Android APP 是否强制更新'),
    ('mobile_app_android_release_notes', '', 'Android APP 更新说明'),
    ('mobile_app_ios_enabled', 'false', 'iOS APP 更新检查开关'),
    ('mobile_app_ios_latest_version', '0.1.0', 'iOS APP 最新版本号'),
    ('mobile_app_ios_latest_build', '1', 'iOS APP 最新构建号，数字越大版本越新'),
    ('mobile_app_ios_package_url', '未配置', 'iOS IPA 安装包下载链接'),
    ('mobile_app_ios_force_update', 'false', 'iOS APP 是否强制更新'),
    ('mobile_app_ios_release_notes', '', 'iOS APP 更新说明')
ON CONFLICT (key) DO NOTHING;
