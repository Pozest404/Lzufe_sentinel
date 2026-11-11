package com.lancai.sentinel.core;

// [OWNER: LEAD] Android 端配置：只保存用户可改的少量参数，分析/存储/网络全部在 Rust。
// 每次调用 JNI 时把配置序列化成 JSON 传过去，Java 不做任何业务计算。

import android.content.Context;
import android.content.SharedPreferences;

import org.json.JSONObject;

import java.io.File;

public final class Config {

    private static final String SP = "lancai_sentinel";
    private static final String K_THRESHOLD = "threshold";
    private static final String K_EXTRA = "keyword_extra";
    private static final String K_AI_ON = "ai_enabled";
    private static final String K_AI_KEY = "deepseek_key";
    private static final String K_AI_MODEL = "deepseek_model";
    private static final String K_AI_PROVIDER = "ai_provider";
    private static final String K_AI_BASE = "ai_base_url";
    private static final String K_PUSH_ON = "push_enabled";
    private static final String K_PUSH_TOKEN = "pushplus_token";
    private static final String K_DEKT_URL = "dekt_url";
    private static final String K_REMIND_ON = "remind_enabled";
    private static final String K_PROFILE_NAME = "profile_name";
    private static final String K_PROFILE_SID = "profile_sid";
    private static final String K_PROFILE_PHONE = "profile_phone";

    /** 默认仍是 DeepSeek，老用户升级后行为不变。 */
    public static final String DEFAULT_AI_PROVIDER = "deepseek";
    public static final String DEFAULT_AI_BASE = "https://api.deepseek.com/chat/completions";
    public static final String DEFAULT_AI_MODEL = "deepseek-chat";
    /** 九学期（第二课堂）默认入口。 */
    public static final String DEFAULT_DEKT_URL = "https://win.9xueqi.com/#/home";

    private static volatile Config INSTANCE;

    private final SharedPreferences p;

    private Config(Context c) {
        p = c.getSharedPreferences(SP, Context.MODE_PRIVATE);
    }

    /** 进程内复用同一份 Config：避免每次 JNI 调用都 new 对象 + 重复访问 SharedPreferences。
     *  SharedPreferences 本身是进程内单例，但 Config 的每次 new 都包装一层 JSONObject 构建。 */
    public static Config get(Context c) {
        Config inst = INSTANCE;
        if (inst == null) {
            synchronized (Config.class) {
                inst = INSTANCE;
                if (inst == null) {
                    INSTANCE = inst = new Config(c.getApplicationContext());
                }
            }
        }
        return inst;
    }

    /** Rust 侧的数据目录：应用私有 files/lancai。 */
    public static String dataDir(Context c) {
        File d = new File(c.getFilesDir(), "lancai");
        if (!d.exists()) {
            //noinspection ResultOfMethodCallIgnored
            d.mkdirs();
        }
        return d.getAbsolutePath();
    }

    public int getThreshold() {
        return p.getInt(K_THRESHOLD, 55);
    }

    public void setThreshold(int v) {
        p.edit().putInt(K_THRESHOLD, v).apply();
    }

    public String getKeywordExtra() {
        return p.getString(K_EXTRA, "");
    }

    public void setKeywordExtra(String v) {
        p.edit().putString(K_EXTRA, v == null ? "" : v).apply();
    }

    public boolean isAiEnabled() {
        return p.getBoolean(K_AI_ON, false);
    }

    public void setAiEnabled(boolean v) {
        p.edit().putBoolean(K_AI_ON, v).apply();
    }

    public String getAiKey() {
        return p.getString(K_AI_KEY, "");
    }

    public void setAiKey(String v) {
        p.edit().putString(K_AI_KEY, v == null ? "" : v.trim()).apply();
    }

    public String getAiModel() {
        return p.getString(K_AI_MODEL, DEFAULT_AI_MODEL);
    }

    public void setAiModel(String v) {
        p.edit().putString(K_AI_MODEL,
                v == null || v.trim().isEmpty() ? DEFAULT_AI_MODEL : v.trim()).apply();
    }

    public String getAiProvider() {
        return p.getString(K_AI_PROVIDER, DEFAULT_AI_PROVIDER);
    }

    public void setAiProvider(String v) {
        p.edit().putString(K_AI_PROVIDER,
                v == null || v.trim().isEmpty() ? DEFAULT_AI_PROVIDER : v.trim()).apply();
    }

    public String getAiBaseUrl() {
        return p.getString(K_AI_BASE, DEFAULT_AI_BASE);
    }

    public void setAiBaseUrl(String v) {
        p.edit().putString(K_AI_BASE,
                v == null || v.trim().isEmpty() ? DEFAULT_AI_BASE : v.trim()).apply();
    }

    public String getDektUrl() {
        return p.getString(K_DEKT_URL, DEFAULT_DEKT_URL);
    }

    public void setDektUrl(String v) {
        p.edit().putString(K_DEKT_URL,
                v == null || v.trim().isEmpty() ? DEFAULT_DEKT_URL : v.trim()).apply();
    }

    public boolean isRemindEnabled() {
        return p.getBoolean(K_REMIND_ON, true);
    }

    public void setRemindEnabled(boolean v) {
        p.edit().putBoolean(K_REMIND_ON, v).apply();
    }

    public boolean isPushEnabled() {
        return p.getBoolean(K_PUSH_ON, false);
    }

    public void setPushEnabled(boolean v) {
        p.edit().putBoolean(K_PUSH_ON, v).apply();
    }

    public String getPushToken() {
        return p.getString(K_PUSH_TOKEN, "");
    }

    public void setPushToken(String v) {
        p.edit().putString(K_PUSH_TOKEN, v == null ? "" : v.trim()).apply();
    }

    public boolean hasAiKey() {
        return !getAiKey().isEmpty();
    }

    public boolean hasPushToken() {
        return !getPushToken().isEmpty();
    }

    // ---- 自动报名时填写的个人信息（只保存本机） ----

    public String getProfileName() {
        return p.getString(K_PROFILE_NAME, "");
    }

    public String getProfileSid() {
        return p.getString(K_PROFILE_SID, "");
    }

    public String getProfilePhone() {
        return p.getString(K_PROFILE_PHONE, "");
    }

    public void setProfile(String name, String sid, String phone) {
        p.edit()
                .putString(K_PROFILE_NAME, name == null ? "" : name.trim())
                .putString(K_PROFILE_SID, sid == null ? "" : sid.trim())
                .putString(K_PROFILE_PHONE, phone == null ? "" : phone.trim())
                .apply();
    }

    /** 传给 Rust 的 JSON；字段名与 config.rs 对应。 */
    public String toJson() {
        try {
            JSONObject root = new JSONObject();
            root.put("analysis", new JSONObject()
                    .put("threshold", getThreshold())
                    .put("keyword_extra", getKeywordExtra()));
            root.put("ai", new JSONObject()
                    .put("enabled", isAiEnabled())
                    .put("api_key", getAiKey())
                    .put("model", getAiModel())
                    .put("base_url", getAiBaseUrl()));
            root.put("pushplus", new JSONObject()
                    .put("enabled", isPushEnabled())
                    .put("token", getPushToken()));
            return root.toString();
        } catch (Throwable t) {
            return "{}";
        }
    }

    public static String mask(String s) {
        if (s == null || s.isEmpty()) return "未填";
        if (s.length() <= 8) return "****";
        return s.substring(0, 4) + "****" + s.substring(s.length() - 4);
    }
}
