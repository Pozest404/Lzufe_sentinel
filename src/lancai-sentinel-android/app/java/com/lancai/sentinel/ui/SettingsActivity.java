package com.lancai.sentinel.ui;

// [OWNER: UI] 设置页：只保留用户真正需要调的参数。
// 分析阈值、追加关键词、多模型 AI、PushPlus、第二课堂网址；保存后立即生效。

import android.app.Activity;
import android.os.Bundle;
import android.text.InputType;
import android.view.View;
import android.view.ViewGroup;
import android.widget.AdapterView;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.Switch;

import com.lancai.sentinel.core.Config;
import com.lancai.sentinel.core.NotificationHelper;
import com.lancai.sentinel.core.ReminderScheduler;

public class SettingsActivity extends Activity {

    private static final int PROVIDER_CUSTOM = 5;
    private static final String[] PROVIDER_NAMES = {
            "DeepSeek", "OpenAI", "通义千问", "智谱 GLM", "Kimi（Moonshot）", "自定义 / 本地"
    };
    private static final String[] PROVIDER_IDS = {
            "deepseek", "openai", "qwen", "glm", "kimi", "custom"
    };
    private static final String[] PROVIDER_URLS = {
            Config.DEFAULT_AI_BASE,
            "https://api.openai.com/v1/chat/completions",
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
            "https://open.bigmodel.cn/api/paas/v4/chat/completions",
            "https://api.moonshot.cn/v1/chat/completions",
            ""
    };
    private static final String[] PROVIDER_MODELS = {
            Config.DEFAULT_AI_MODEL,
            "gpt-4o-mini",
            "qwen-plus",
            "glm-4-flash",
            "moonshot-v1-8k",
            ""
    };

    private EditText etThreshold, etExtra, etAiKey, etAiBase, etAiModel, etPushToken, etDektUrl;
    private EditText etName, etSid, etPhone;
    private Switch swAi, swPush, swRemind;
    private Spinner spProvider;
    private int lastProviderPos;
    private Config cfg;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        cfg = Config.get(this);
        render();
    }

    private void render() {
        LinearLayout page = UiKit.vertical(this);
        page.setBackgroundColor(UiKit.BG);
        page.addView(UiKit.topBar(this, "设置"), new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        LinearLayout root = UiKit.vertical(this);
        root.setPadding(dp(14), dp(8), dp(14), dp(26));
        scroll.addView(root, new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        page.addView(scroll, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        setContentView(page);

        root.addView(UiKit.hint(this, "改完点底部「保存」。分析、存储与请求都由 Rust 核心执行。"));

        // ---- 分析规则 ----
        LinearLayout rules = UiKit.card(this);
        rules.addView(UiKit.section(this, "分析与规则"));
        rules.addView(UiKit.hint(this, "重要线越低越灵敏；默认 55 分。"));

        etThreshold = numberInput("重要线（10-100，默认 55）");
        etThreshold.setText(String.valueOf(cfg.getThreshold()));
        rules.addView(etThreshold);

        etExtra = textInput("追加关键词，逗号分隔（可选）");
        etExtra.setText(cfg.getKeywordExtra());
        rules.addView(etExtra);
        rules.addView(UiKit.hint(this, "例如：素拓, 到梦空间。每个 +8 分，最多 +16。"));
        addCard(root, rules);

        // ---- AI ----
        LinearLayout ai = UiKit.card(this);
        ai.addView(UiKit.section(this, "AI 总结（可选，支持多模型）"));
        swAi = switchRow(ai, "命中的重要消息自动总结");
        swAi.setChecked(cfg.isAiEnabled());

        ai.addView(UiKit.label(this, "服务商预设"));
        spProvider = new Spinner(this);
        ArrayAdapter<String> adapter = new ArrayAdapter<>(this,
                android.R.layout.simple_spinner_item, PROVIDER_NAMES);
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
        spProvider.setAdapter(adapter);
        lastProviderPos = providerIndex(cfg.getAiProvider());
        spProvider.setSelection(lastProviderPos);
        spProvider.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            @Override
            public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                // 初始化时的回调不应覆盖已保存的 Base URL / 模型。
                if (position == lastProviderPos) return;
                lastProviderPos = position;
                applyPreset(position);
            }

            @Override
            public void onNothingSelected(AdapterView<?> parent) {
            }
        });
        ai.addView(spProvider);

        etAiKey = secretInput("API Key（本地/免密接口可留空）");
        etAiKey.setText(cfg.getAiKey());
        ai.addView(etAiKey);

        etAiBase = textInput("Base URL（OpenAI 兼容接口）");
        etAiBase.setText(cfg.getAiBaseUrl());
        ai.addView(etAiBase);

        etAiModel = textInput("模型名，例如 deepseek-chat / gpt-4o-mini");
        etAiModel.setText(cfg.getAiModel());
        ai.addView(etAiModel);
        ai.addView(UiKit.hint(this, "支持任何 OpenAI Chat Completions 兼容接口："
                + "只填到 /v1 会自动补 /chat/completions；本地 Ollama 选「自定义」。"
                + "不填 Key 也能用本地规则分析，只是没有 AI 总结。"));
        addCard(root, ai);

        // ---- Push ----
        LinearLayout push = UiKit.card(this);
        push.addView(UiKit.section(this, "PushPlus 微信推送（可选）"));
        swPush = switchRow(push, "重要消息自动推送到微信");
        swPush.setChecked(cfg.isPushEnabled());

        etPushToken = secretInput("PushPlus Token");
        etPushToken.setText(cfg.getPushToken());
        push.addView(etPushToken);
        push.addView(UiKit.hint(this, "在 pushplus.plus 登录后获取 Token；不填则只在本机提示。"));
        addCard(root, push);

        // ---- 校园服务 ----
        LinearLayout campus = UiKit.card(this);
        campus.addView(UiKit.section(this, "校园服务"));
        etDektUrl = textInput("第二课堂网址");
        etDektUrl.setText(cfg.getDektUrl());
        campus.addView(etDektUrl);
        campus.addView(UiKit.hint(this, "首页与菜单里的「第二课堂」会在应用内打开这个地址；"
                + "默认是九学期 win.9xueqi.com，可换成学校自己的入口。"));
        addCard(root, campus);

        // ---- 报名信息（自动报名时填写） ----
        LinearLayout profile = UiKit.card(this);
        profile.addView(UiKit.section(this, "报名信息（可选）"));
        profile.addView(UiKit.hint(this, "仅在你在日程里确认「自动报名」后，"
                + "打开报名页时自动填写，不会上传。"));
        etName = textInput("姓名");
        etName.setText(cfg.getProfileName());
        etSid = textInput("学号");
        etSid.setText(cfg.getProfileSid());
        etPhone = textInput("手机号");
        etPhone.setText(cfg.getProfilePhone());
        profile.addView(etName);
        profile.addView(etSid);
        profile.addView(etPhone);
        addCard(root, profile);

        // ---- 提醒通知 ----
        LinearLayout notify = UiKit.card(this);
        notify.addView(UiKit.section(this, "提醒通知"));
        swRemind = switchRow(notify, "为日程与第二课堂活动发送系统通知");
        swRemind.setChecked(cfg.isRemindEnabled());
        notify.addView(UiKit.hint(this, "在「日程与待办」里给每条设置提前时间。"
                + "Android 13 及以上首次使用需允许通知权限；关闭后不再排期提醒。"));
        addCard(root, notify);

        Button save = UiKit.button(this, "保存设置", UiKit.brand(this),
                new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        save();
                    }
                });
        LinearLayout.LayoutParams sp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(46));
        sp.topMargin = dp(14);
        root.addView(save, sp);
    }

    private void save() {
        int threshold;
        try {
            String raw = etThreshold.getText().toString().trim();
            threshold = Integer.parseInt(raw.isEmpty() ? "55" : raw);
        } catch (Throwable t) {
            threshold = -1;
        }
        if (threshold < 10 || threshold > 100) {
            etThreshold.setError("请填 10-100 的整数");
            etThreshold.requestFocus();
            UiKit.toastLong(this, "重要线要在 10-100 之间");
            return;
        }
        int providerPos = spProvider.getSelectedItemPosition();
        if (providerPos < 0) providerPos = 0;
        String aiKey = etAiKey.getText().toString().trim();
        String aiBase = etAiBase.getText().toString().trim();
        String aiModel = etAiModel.getText().toString().trim();
        String pushToken = etPushToken.getText().toString().trim();
        String dektUrl = etDektUrl.getText().toString().trim();
        if (swAi.isChecked()) {
            if (aiBase.isEmpty()) {
                etAiBase.setError("请填写 Base URL");
                etAiBase.requestFocus();
                UiKit.toastLong(this, "已开启 AI，但没填 Base URL");
                return;
            }
            if (aiModel.isEmpty()) {
                etAiModel.setError("请填写模型名");
                etAiModel.requestFocus();
                UiKit.toastLong(this, "已开启 AI，但没填模型名");
                return;
            }
            if (aiKey.isEmpty() && providerPos != PROVIDER_CUSTOM) {
                UiKit.toastLong(this, "该服务商通常需要 API Key；本地/免密接口请选「自定义」");
                etAiKey.requestFocus();
                return;
            }
        }
        if (swPush.isChecked() && pushToken.isEmpty()) {
            UiKit.toastLong(this, "已开启推送，但没填 PushPlus Token");
            etPushToken.requestFocus();
            return;
        }
        if (dektUrl.isEmpty()) dektUrl = Config.DEFAULT_DEKT_URL;
        if (!dektUrl.startsWith("http://") && !dektUrl.startsWith("https://")) {
            dektUrl = "https://" + dektUrl;
        }
        cfg.setThreshold(threshold);
        cfg.setKeywordExtra(etExtra.getText().toString().trim());
        cfg.setAiEnabled(swAi.isChecked());
        cfg.setAiProvider(PROVIDER_IDS[providerPos]);
        cfg.setAiKey(aiKey);
        cfg.setAiBaseUrl(aiBase);
        cfg.setAiModel(aiModel);
        cfg.setPushEnabled(swPush.isChecked());
        cfg.setPushToken(pushToken);
        cfg.setDektUrl(dektUrl);
        cfg.setRemindEnabled(swRemind.isChecked());
        cfg.setProfile(etName.getText().toString(), etSid.getText().toString(),
                etPhone.getText().toString());
        NotificationHelper.ensureChannel(this);
        final android.content.Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                ReminderScheduler.reschedule(ctx);
            }
        }, "lancai-remind-settings").start();
        UiKit.toast(this, "已保存，下次分析生效");
        finish();
    }

    // ------------------------------------------------------------------

    /** 已保存的服务商 id → 下拉框下标；未知值按「自定义」处理。 */
    private int providerIndex(String id) {
        for (int i = 0; i < PROVIDER_IDS.length; i++) {
            if (PROVIDER_IDS[i].equals(id)) return i;
        }
        return PROVIDER_CUSTOM;
    }

    /** 选中预设时填入默认地址与模型；「自定义 / 本地」不覆盖用户输入。 */
    private void applyPreset(int position) {
        if (position < 0 || position >= PROVIDER_URLS.length || position == PROVIDER_CUSTOM) {
            return;
        }
        etAiBase.setText(PROVIDER_URLS[position]);
        etAiModel.setText(PROVIDER_MODELS[position]);
    }

    private EditText numberInput(String hint) {
        EditText e = textInput(hint);
        e.setInputType(InputType.TYPE_CLASS_NUMBER);
        return e;
    }

    private EditText textInput(String hint) {
        EditText e = new EditText(this);
        e.setHint(hint);
        e.setSingleLine(true);
        e.setTextSize(14);
        e.setTextColor(UiKit.TEXT);
        e.setHintTextColor(0xFF9AA8A2);
        return e;
    }

    private EditText secretInput(String hint) {
        EditText e = textInput(hint);
        e.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        return e;
    }

    private Switch switchRow(LinearLayout parent, String label) {
        LinearLayout row = UiKit.horizontal(this);
        row.setPadding(0, dp(8), 0, dp(2));
        row.addView(UiKit.text(this, label, 13, UiKit.TEXT),
                new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        Switch sw = new Switch(this);
        sw.setChecked(false);
        row.addView(sw);
        parent.addView(row);
        return sw;
    }

    private void addCard(LinearLayout root, LinearLayout card) {
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        p.topMargin = dp(10);
        root.addView(card, p);
    }

    private int dp(float v) {
        return UiKit.dp(this, v);
    }
}
