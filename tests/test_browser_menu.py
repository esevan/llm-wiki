import json
from pathlib import Path

import pytest

playwright = pytest.importorskip("playwright.sync_api")


def test_locale_switch_translates_static_ui_preserves_input_and_avoids_ai_calls() -> None:
    from playwright.sync_api import sync_playwright

    root = Path(__file__).parents[1]
    resources = {
        locale: json.loads((root / "llm_wiki" / "static" / "i18n" / f"{locale}.json").read_text(encoding="utf-8"))
        for locale in ("en", "ko")
    }
    page_url = (root / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        requested: list[str] = []

        def route_api(route) -> None:
            url = route.request.url
            requested.append(url)
            if "/api/i18n/en" in url:
                payload = resources["en"]
            elif "/api/i18n/ko" in url:
                payload = resources["ko"]
            elif "/api/settings/locale" in url:
                payload = {"locale": "en", "explicit": True, "supported_locales": ["ko", "en"]}
            elif "/api/board" in url:
                payload = {"captures": [], "problems": [], "features": []}
            elif "/api/dashboard" in url:
                payload = {"goals": [], "events": []}
            elif "/api/provider/config" in url:
                payload = {
                    "base_url": "",
                    "model": "",
                    "advanced_model": "",
                    "advanced_tasks": {},
                    "api_key_configured": False,
                }
            elif "/api/transitions" in url:
                payload = {"transitions": []}
            elif "/recent-archive" in url:
                payload = {"documents": []}
            elif "/completed-solutions" in url:
                payload = {"solutions": []}
            else:
                payload = {}
            route.fulfill(status=200, content_type="application/json", body=json.dumps(payload))

        page.route("**/api/**", route_api)
        page.goto(page_url)
        page.evaluate(
            "resources => { localeResources=resources; rebuildLocaleReverse(); activeLocale='en'; applyLocale(); }",
            resources,
        )
        page.locator("#capture-text").fill("사용자가 작성한 원문")
        page.locator("#manual-modal").evaluate("dialog => dialog.showModal()")
        page.locator("#manual-title").fill("편집 중인 제목")
        elapsed = page.evaluate(
            "async () => { const start=performance.now(); await setLocale('ko', false); return performance.now()-start }"
        )

        assert page.locator("html").get_attribute("lang") == "ko"
        assert page.get_by_role("button", name="Vault 검색", exact=False).count() == 1
        assert page.locator("#capture-text").input_value() == "사용자가 작성한 원문"
        assert page.locator("#manual-modal").evaluate("dialog => dialog.open")
        assert page.locator("#manual-title").input_value() == "편집 중인 제목"
        assert elapsed < 100
        assert not any(
            fragment in url for url in requested for fragment in ("/chat", "/draft", "/refine", "/conflict-review")
        )
        browser.close()


def test_saved_locale_initialization_reloads_stored_content_with_the_database_locale() -> None:
    from playwright.sync_api import sync_playwright

    root = Path(__file__).parents[1]
    resources = {
        locale: json.loads((root / "llm_wiki" / "static" / "i18n" / f"{locale}.json").read_text(encoding="utf-8"))
        for locale in ("en", "ko")
    }
    page_url = (root / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        result = page.evaluate(
            """async resources => {
                const requests=[];
                window.fetch=(url,options={})=>{
                    const value=String(url),headers=options.headers||{};
                    requests.push({value,locale:headers['X-LLM-Wiki-Locale']||'',cache:options.cache||''});
                    let payload={};
                    if(value.includes('/settings/locale'))payload={locale:'ko',explicit:true,supported_locales:['ko','en']};
                    else if(value.endsWith('/i18n/en'))payload=resources.en;
                    else if(value.endsWith('/i18n/ko'))payload=resources.ko;
                    else if(value.endsWith('/board'))payload={captures:[],problems:[],features:[]};
                    else if(value.endsWith('/dashboard'))payload={goals:[],events:[]};
                    return Promise.resolve({ok:true,status:200,json:async()=>payload,text:async()=>''});
                };
                await initializeLocale();
                return {locale:activeLocale,select:document.querySelector('#locale-select').value,requests};
            }""",
            resources,
        )
        assert result["locale"] == "ko"
        assert result["select"] == "ko"
        assert any(item["value"].endswith("/board") and item["locale"] == "ko" for item in result["requests"])
        assert any(item["value"].endswith("/dashboard") and item["locale"] == "ko" for item in result["requests"])
        assert all(
            item["cache"] == "no-store"
            for item in result["requests"]
            if "/i18n/" in item["value"] or "/settings/locale" in item["value"]
        )
        browser.close()


def test_queue_explains_each_job_and_formats_background_results() -> None:
    from playwright.sync_api import sync_playwright

    root = Path(__file__).parents[1]
    page_url = (root / "llm_wiki" / "static" / "index.html").as_uri()
    resources = {
        locale: json.loads((root / "llm_wiki" / "static" / "i18n" / f"{locale}.json").read_text(encoding="utf-8"))
        for locale in ("en", "ko")
    }
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        result = page.evaluate(
            """async () => {
                activeLocale='en';
                window.boardItems={'features:feature-1':{title:'Make Queue readable'}};
                queueJobs=[
                    {
                        id:'review-1',task_kind:'completion_review',entity_type:'features',entity_id:'feature-1',
                        status:'running',progress:{completed:2,total:4},result_interface:'completion_review',
                        error:null,created_at:'2026-09-02T10:00:00Z'
                    },
                    {
                        id:'embedding-1',task_kind:'embedding_refresh',entity_type:'vault',entity_id:'documents',
                        status:'completed',progress:{completed:3,total:3},result_interface:'embedding_coverage',
                        error:null,created_at:'2026-09-02T09:00:00Z',finished_at:'2026-09-02T09:01:00Z'
                    },
                    {
                        id:'comment-translation-1',task_kind:'derived_translation',
                        entity_type:'solution_progress_comments',entity_id:'comment-1',status:'running',
                        progress:{completed:0,total:1},result_interface:'owning_content',error:null,
                        created_at:'2026-09-02T10:01:00Z'
                    }
                ];
                renderQueue();
                const queueText=document.querySelector('#queue-list').innerText;
                const pendingResult=document.querySelector('[data-job-id="review-1"] [data-job-action="result"]');
                queueJobs[0].status='awaiting_review';
                renderQueue();
                const readyResult=document.querySelector('[data-job-id="review-1"] [data-job-action="result"]');
                let notice=null;
                api=async path=>({
                    status:'completed',result_interface:'embedding_coverage',
                    result:{updated:3,coverage:{documents:10,semantic_ready:9}}
                });
                showNotice=(message,title)=>{notice={message,title}};
                await openJobResult('embedding-1');
                return {
                    queueText,notice,
                    pendingResult:{text:pendingResult?.innerText,disabled:pendingResult?.disabled},
                    readyResult:{text:readyResult?.innerText,disabled:readyResult?.disabled}
                };
            }"""
        )

        assert "Completion Review" in result["queueText"]
        assert "Solution · Make Queue readable" in result["queueText"]
        assert "Checks recorded work and validation criteria" in result["queueText"]
        assert "2 of 4 steps" in result["queueText"]
        assert "Result" in result["queueText"]
        assert "Completion Review page" in result["queueText"]
        assert "Solution Work · Comment" in result["queueText"]
        assert "Adds the missing language version" in result["queueText"]
        assert result["pendingResult"] == {"text": "Available when complete", "disabled": True}
        assert result["readyResult"] == {"text": "Open result page →", "disabled": False}
        assert result["notice"] == {
            "title": "Embedding refresh",
            "message": "3 documents updated. Semantic search is ready for 9 of 10 documents.",
        }
        assert "{" not in result["notice"]["message"]
        korean = page.evaluate(
            """resources => {
                localeResources=resources; rebuildLocaleReverse(); activeLocale='ko'; renderQueue();
                return document.querySelector('#queue-list').innerText;
            }""",
            resources,
        )
        assert "완료 검토" in korean
        assert "기록된 작업과 검증 기준을 확인합니다" in korean
        assert "4단계 중 2단계" in korean
        assert "완료 검토 페이지" in korean
        assert "결과 페이지 열기 →" in korean
        assert "Solution Work · 댓글" in korean
        browser.close()


def test_conflict_review_cards_require_independent_resolutions_and_acceptance_rationale() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 900, "height": 700})
        page.goto(page_url)
        elapsed = page.evaluate(
            """() => {
                const conflicts=Array.from({length:20},(_,index)=>({
                    id:'conflict-'+(index+1),target_id:'adr-'+index,target_title:'ADR-'+index,
                    severity:['high','medium','low'][index%3],category:'Storage ownership '+index,
                    summary:'The claims differ.',current_claim:'Client state is authoritative.',
                    existing_claim:'Server state is authoritative.',impact:'Clients can diverge.',
                    recommendation:'Keep server authority.',evidence:[{citation:'adr.md:1-2',excerpt:'Server owns state.'}]
                }));
                const start=performance.now();
                document.querySelector('#item-detail-notes').innerHTML=conflictReviewMarkup({
                    run_id:'run-1',feature_id:'feature-1',recommended_state:'potential_conflict',conflicts
                },'feature-1',false);
                document.querySelector('#item-detail-type').textContent='Conflict review · your decision required';
                document.querySelector('#item-detail-modal').showModal();
                return performance.now()-start;
            }"""
        )
        assert elapsed < 100
        assert page.locator(".conflict-card").count() == 20
        assert page.locator(".conflict-severity").first.inner_text() == "HIGH"
        severity_colors = page.locator(".conflict-card").evaluate_all(
            "cards => cards.slice(0,3).map(card => getComputedStyle(card).borderLeftColor)"
        )
        assert len(set(severity_colors)) == 3
        assert page.locator(".conflict-card").first.get_by_text("Client state is authoritative.").is_visible()
        assert page.locator(".conflict-card details").first.get_by_text("Server owns state.").count() == 1
        assert page.locator("#conflict-review-summary").inner_text() == "20 conflicts · 0 resolved · 20 unresolved"
        continue_button = page.locator("#conflict-review-continue")
        assert continue_button.is_disabled()

        first = page.locator(".conflict-card").nth(0)
        first.get_by_label("Apply recommendation").check()
        assert page.locator("#conflict-review-summary").inner_text() == "20 conflicts · 1 resolved · 19 unresolved"
        second = page.locator(".conflict-card").nth(1)
        second.get_by_label("Accept conflict").check()
        assert second.locator("textarea").get_attribute("required") == ""
        assert page.locator("#conflict-review-summary").inner_text() == "20 conflicts · 1 resolved · 19 unresolved"
        second.locator("textarea").fill("Offline-first is intentional.")
        assert page.locator("#conflict-review-summary").inner_text() == "20 conflicts · 2 resolved · 18 unresolved"
        assert first.get_by_label("Apply recommendation").is_checked()
        assert first.locator("input[type=radio]:checked").count() == 1
        footer = page.locator(".conflict-review-footer")
        assert footer.evaluate("node => getComputedStyle(node).position") == "sticky"
        page.locator(".conflict-card").nth(2).get_by_label("Apply recommendation").check()
        page.evaluate(
            """() => {
                document.querySelectorAll('.conflict-card').forEach(card => {
                    if (!card.querySelector('input[type=radio]:checked')) {
                        card.querySelector('input[value=apply_recommendation]').checked=true;
                    }
                });
                updateConflictResolutionState();
                api=async()=>{throw Error('The decisions could not be saved.')};
            }"""
        )
        assert not continue_button.is_disabled()
        continue_button.click()
        page.locator("#conflict-review-error").get_by_text("The decisions could not be saved.").wait_for()
        assert page.locator("#item-detail-modal").evaluate("dialog => dialog.open")
        assert first.get_by_label("Apply recommendation").is_checked()

        page.locator("#item-detail-notes").evaluate(
            """notes => notes.innerHTML=conflictReviewMarkup({run_id:'legacy-run',findings:[{
                severity:'high',claim:'Legacy current claim',path:'legacy.md',excerpt:'Legacy existing claim',
                explanation:'Legacy explanation',required_resolution:'Run a current review'
            }]},'feature-1',false)"""
        )
        assert page.locator(".conflict-card").count() == 1
        assert page.locator(".conflict-card").get_by_role("heading", name="Legacy current claim").is_visible()
        assert page.locator("#conflict-review-continue").is_disabled()
        assert page.get_by_text("run a fresh review to save item resolutions", exact=False).is_visible()

        page.locator("#item-detail-notes").evaluate(
            """notes => notes.innerHTML=conflictReviewMarkup({
                recommended_state:'clear',summary:'No conflicts found.',findings:[],scope:{documents:1,semantic_ready:1}
            },'feature-1',false)"""
        )
        assert page.locator(".conflict-card").count() == 0
        assert page.get_by_role("button", name="Declare clear").is_visible()
        browser.close()


def test_knowledge_reader_opens_immediately_and_reacts_to_async_completion() -> None:
    from playwright.sync_api import sync_playwright

    root = Path(__file__).parents[1]
    resources = {
        locale: json.loads((root / "llm_wiki" / "static" / "i18n" / f"{locale}.json").read_text(encoding="utf-8"))
        for locale in ("en", "ko")
    }
    page_url = (root / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        page.evaluate(
            """resources => {
                localeResources=resources;rebuildLocaleReverse();activeLocale='en';
                window.fetch=(url)=>String(url).includes('/knowledge?')
                    ? new Promise(resolve=>{window.__finishKnowledge=()=>resolve({ok:true,status:200,json:async()=>({
                        translated:false,canonical_locale:'en',cache_status:'not_applicable',markdown:'# Canonical result'
                    }),text:async()=>''})})
                    : Promise.resolve({ok:true,status:200,json:async()=>({}),text:async()=>''});
                void searchArchivedDocument('Knowledge/result.md');
            }""",
            resources,
        )
        assert page.locator("#item-detail-modal").evaluate("dialog => dialog.open")
        assert page.locator("#item-detail-modal").get_attribute("aria-busy") == "true"
        page.get_by_text("Opening canonical Knowledge", exact=True).wait_for(timeout=1000)
        page.evaluate("window.__finishKnowledge()")
        page.get_by_text("# Canonical result", exact=True).wait_for(timeout=1000)
        assert page.locator("#item-detail-modal").get_attribute("aria-busy") == "false"
        browser.close()


def test_korean_knowledge_reader_detaches_from_durable_job_when_surface_closes() -> None:
    source = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").read_text(encoding="utf-8")

    assert "translate=false" in source
    assert "knowledge-progress'+(active?' is-active':'')" in source
    assert "result.cache_status==='pending'" in source
    assert "position:sticky" in source
    assert "new AbortController()" in source
    assert "detachKnowledgeTranslationReader" in source
    detach_handler = source.split("detachKnowledgeTranslationReader=function", 1)[1].split(";\n", 1)[0]
    assert "/cancel" not in detach_handler
    assert "waitForJob(job.id,controller.signal,false)" in source
    assert "data-retry-knowledge" in source


def test_given_running_knowledge_translation_when_reader_closes_then_job_continues() -> None:
    """Closing the reader detaches UI polling without cancelling durable translation work."""
    from playwright.sync_api import sync_playwright

    root = Path(__file__).parents[1]
    resources = {
        locale: json.loads((root / "llm_wiki" / "static" / "i18n" / f"{locale}.json").read_text(encoding="utf-8"))
        for locale in ("en", "ko")
    }
    page_url = (root / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        page.evaluate(
            """resources => {
                localeResources=resources;rebuildLocaleReverse();activeLocale='ko';
                window.__knowledgeRequests=[];
                window.fetch=(url,options={})=>{
                    const value=String(url),method=options.method||'GET';
                    window.__knowledgeRequests.push({url:value,method});
                    let payload={};
                    if(value.includes('/knowledge/translate?')){
                        payload={id:'translation-1',status:'queued',progress:{completed:0,total:2}};
                    }else if(value.includes('/knowledge?')){
                        payload={translated:false,canonical_locale:'en',cache_status:'pending',markdown:'# Canonical'};
                    }else if(value.endsWith('/jobs/translation-1')){
                        payload={id:'translation-1',status:'running',progress:{completed:0,total:2}};
                    }
                    return Promise.resolve({ok:true,status:200,json:async()=>payload,text:async()=>''});
                };
                void searchArchivedDocument('Knowledge/result.md');
            }""",
            resources,
        )
        page.wait_for_function("activeKnowledgeTranslation?.jobId === 'translation-1'")

        page.locator("#item-detail-close").click()

        page.wait_for_function("activeKnowledgeTranslation === null")
        requests = page.evaluate("window.__knowledgeRequests")
        assert not [request for request in requests if request["url"].endswith("/jobs/translation-1/cancel")]
        assert not page.locator("#item-detail-modal").evaluate("dialog => dialog.open")
        browser.close()


def test_removed_product_stage_has_no_browser_surface() -> None:
    source = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").read_text()
    forbidden = ("model-tasks", "board.tasks", "newTask", "Explore next task", "Task proposal")
    assert all(marker not in source for marker in forbidden)


def test_ai_setup_exposes_lineage_interpretation_model_routing() -> None:
    source = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").read_text()
    assert 'data-advanced-task="lineage_inference"' in source
    assert "Lineage interpretation" in source


def test_navigation_switches_views_in_a_real_browser() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        page.locator('[data-view="search"]').click()
        assert "active" in page.locator("#search").get_attribute("class")
        assert "active" not in page.locator("#workbench").get_attribute("class")
        page.locator('[data-view="compass"]').click()
        assert "active" in page.locator("#compass").get_attribute("class")
        browser.close()


def test_cards_do_not_render_duplicate_explore_buttons() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.goto(page_url)
        page.evaluate(
            """() => {
                const capture = {id: 'capture-1', text: 'Capture opens from its card', category: 'General'};
                const problem = {id: 'problem-1', statement: 'Problem opens from its card', detail: '', state: 'approved', category: 'General'};
                const proposed = {id: 'solution-1', problem_id: problem.id, title: 'Proposed Solution', outcome: 'Open from the card', state: 'proposed', conflict_state: 'clear', category: 'General'};
                const active = {id: 'solution-2', problem_id: problem.id, title: 'Active Solution', outcome: 'Open from the card', state: 'approved', conflict_state: 'clear', category: 'General'};
                const board = {captures: [capture], problems: [problem], features: [proposed, active], tasks: []};
                window.workbenchBoard = board;
                window.boardItems = Object.fromEntries(Object.entries(board).flatMap(([type, items]) => items.map(item => [type + ':' + item.id, item])));
                document.querySelector('#board').innerHTML = renderSwimlane('General', board);
            }"""
        )

        assert page.get_by_role("button", name="Explore", exact=True).count() == 0
        assert page.get_by_role("button", name="Explore as Problem", exact=True).count() == 0
        assert page.get_by_role("button", name="Explore next solution", exact=True).count() == 1
        assert page.locator('[data-item-type="captures"]').get_attribute("tabindex") == "0"
        assert page.locator('[data-item-type="problems"]').get_attribute("tabindex") == "0"
        assert page.locator('[data-item-type="features"]').count() == 2
        browser.close()


def test_edit_manually_opens_and_saves_text_with_quotes_and_newlines() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page_errors: list[str] = []
        page.on("pageerror", lambda error: page_errors.append(str(error)))
        page.goto(page_url)
        page.evaluate(
            r"""() => {
                const problem = {
                    id: 'problem-1',
                    statement: "Owner's broken manual edit",
                    detail: "First line\nSecond line with 'quoted context'",
                    state: 'draft',
                    category: 'General',
                    attention_rank: 0,
                    manual_priority: 0
                };
                const board = {captures: [], problems: [problem], features: []};
                window.workbenchBoard = board;
                window.boardItems = {'problems:problem-1': problem};
                document.querySelector('#board').innerHTML = renderSwimlane('General', board);
                window.manualRequests = [];
                window.fetch = async (url, options = {}) => {
                    const path = String(url);
                    if (path.includes('/api/items/problems/problem-1')) {
                        window.manualRequests.push({method: options.method, body: options.body});
                        return new Response(null, {status: 204});
                    }
                    if (path.includes('/api/board')) {
                        return new Response(JSON.stringify(board), {status: 200, headers: {'Content-Type': 'application/json'}});
                    }
                    if (path.includes('/recent-archive')) {
                        return new Response(JSON.stringify({documents: []}), {status: 200, headers: {'Content-Type': 'application/json'}});
                    }
                    if (path.includes('/completed-solutions')) {
                        return new Response(JSON.stringify({solutions: []}), {status: 200, headers: {'Content-Type': 'application/json'}});
                    }
                    return new Response(JSON.stringify({}), {status: 200, headers: {'Content-Type': 'application/json'}});
                };
            }"""
        )
        page_errors.clear()

        page.locator(".card-menu summary").click()
        page.get_by_role("button", name="Edit manually").click()

        assert page.locator("#manual-modal").evaluate("dialog => dialog.open")
        assert page.locator("#manual-title").input_value() == "Owner's broken manual edit"
        assert page.locator("#manual-detail").input_value() == "First line\nSecond line with 'quoted context'"

        page.locator("#manual-title").fill("Updated owner's problem")
        page.locator("#manual-detail").fill("Updated line one\nUpdated line two")
        page.get_by_role("button", name="Save manually").click()
        page.wait_for_function("window.manualRequests.length === 1")

        request = page.evaluate("window.manualRequests[0]")
        assert request["method"] == "PUT"
        assert request["body"] == '{"title":"Updated owner\'s problem","detail":"Updated line one\\nUpdated line two"}'
        assert not page.locator("#manual-modal").evaluate("dialog => dialog.open")
        assert page_errors == []
        browser.close()


def test_complete_problem_button_shows_spinner_while_request_is_pending() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        page.locator("#item-detail-notes").evaluate(
            """notes => {
                notes.innerHTML = '<footer><button id="test-complete" class="primary icon-action" aria-label="Complete Problem">✓</button></footer>';
                const button = document.querySelector('#test-complete');
                button.onclick = () => confirmProblemCompletion('problem-id', 'review-id', button);
            }"""
        )
        page.evaluate(
            "window.fetch = () => new Promise(resolve => setTimeout(() => resolve({ok: true, status: 200, json: async () => ({})}), 1000))"
        )

        button = page.locator("#test-complete")
        button.dispatch_event("click")
        assert button.is_disabled()
        assert button.get_attribute("aria-busy") == "true"
        assert button.inner_text() == ""
        spinner = button.evaluate(
            "button => ({content: getComputedStyle(button, '::before').content, animation: getComputedStyle(button, '::before').animationName})"
        )
        assert spinner == {"content": '""', "animation": "spin"}
        browser.close()


def test_conflict_review_modal_is_compact_and_decision_icons_align() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.goto(page_url)
        page.locator("#item-detail-type").evaluate(
            "node => node.textContent = 'Conflict review · your decision required'"
        )
        page.locator("#item-detail-title").evaluate("node => node.textContent = 'Conflict review needs attention'")
        page.locator("#item-detail-notes").evaluate(
            """notes => notes.innerHTML = `
                <div class="conflict-report">
                    <div class="detail-block"><dt>Summary</dt><dd>Review the evidence.</dd></div>
                    <div class="detail-block"><dt>Findings</dt><dd>${'Detailed finding. '.repeat(80)}</dd></div>
                </div>
                <footer class="conflict-decision">
                    <textarea aria-label="Decision note"></textarea>
                    <button id="keep-conflicted" class="tiny icon-action">!</button>
                    <button id="declare-clear" class="tiny hot icon-action">✓</button>
                </footer>`"""
        )
        page.locator("#item-detail-modal").evaluate("dialog => dialog.showModal()")

        modal = page.locator("#item-detail-modal").bounding_box()
        notes = page.locator("#item-detail-notes").bounding_box()
        report = page.locator(".conflict-report").bounding_box()
        decision = page.locator(".conflict-decision").bounding_box()
        first = page.locator("#keep-conflicted").bounding_box()
        second = page.locator("#declare-clear").bounding_box()
        assert modal["height"] <= 520
        assert report["y"] + report["height"] <= decision["y"]
        assert decision["y"] + decision["height"] <= notes["y"] + notes["height"]
        assert first["height"] == second["height"] == 36
        assert first["y"] == second["y"]
        browser.close()


def test_conflict_review_is_enqueued_without_opening_a_progress_modal() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.goto(page_url)
        page.evaluate(
            """() => {
                const trigger = document.createElement('button');
                trigger.textContent = '↯';
                document.body.append(trigger);
                window.fetch = () => Promise.resolve({ok: true, status: 202, json: async () => ({id: 'review-job'}), text: async () => ''});
                runConflictReview('preview', trigger);
            }"""
        )

        page.get_by_text("Conflict review queued", exact=True).wait_for(timeout=1000)
        assert not page.locator("#item-detail-modal").evaluate("dialog => dialog.open")
        assert page.evaluate("activeConflictRun") == "review-job"
        browser.close()


def test_solution_proposal_sections_do_not_overlap() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 1280, "height": 720})
        page.goto(page_url)
        page.evaluate(
            """showDraftReview('problems', 'problem-id', {
                title: 'A proposed solution',
                outcome: 'Outcome '.repeat(30),
                non_goals: 'Non-goal '.repeat(30),
                validation_criteria: 'Evidence '.repeat(30)
            })"""
        )

        sections = [
            page.locator(selector).bounding_box()
            for selector in (
                "#draft-main-field",
                "#draft-detail-field",
                "#draft-extra-field",
                "#draft-validation-field",
            )
        ]
        for current, following in zip(sections, sections[1:]):
            assert current["y"] + current["height"] <= following["y"]
        form = page.locator("#draft-form")
        assert form.evaluate("node => node.scrollHeight >= node.clientHeight")
        browser.close()


def test_manual_transition_form_uses_clear_paths_and_short_actions() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 390, "height": 844})
        page.goto(page_url)
        page.evaluate(
            """allTransitions = [{
                id: 'solution_to_approved',
                label: 'Start manually',
                submit_label: 'Start work',
                description: 'Move this Solution to In progress without an AI conflict review.',
                fields: [
                    {name: 'approval_path', label: 'Conflict check', type: 'select', required: true, options: [
                        {value: 'checked', label: 'Already checked'},
                        {value: 'skip', label: 'Skip with a reason'}
                    ]},
                    {name: 'citation', label: 'Review basis', type: 'textarea', required_when: {approval_path: 'checked'}, visible_when: {approval_path: 'checked'}},
                    {name: 'skip_reason', label: 'Skip reason', type: 'textarea', required_when: {approval_path: 'skip'}, visible_when: {approval_path: 'skip'}}
                ]
            }]"""
        )
        page.evaluate("openTransition('solution_to_approved', 'features', 'preview')")

        assert page.locator(".manual-kicker").inner_text() == "MANUAL · AI NOT USED"
        assert page.locator("#transition-submit").inner_text() == "Start work"
        assert page.locator("[name=approval_path] option").all_inner_texts() == [
            "Already checked",
            "Skip with a reason",
        ]
        assert page.locator("[name=citation]").is_visible()
        assert page.locator("[name=citation]").get_attribute("required") == ""
        assert not page.locator("[name=skip_reason]").is_visible()
        required_tag = page.locator(".transition-field:visible .transition-required").first
        assert required_tag.evaluate("tag => getComputedStyle(tag).borderRadius") == "999px"
        assert required_tag.evaluate("tag => getComputedStyle(tag).backgroundColor") == "rgb(255, 240, 246)"
        submit = page.locator("#transition-submit")
        submit.evaluate("button => button.setAttribute('aria-busy', 'true')")
        spinner = submit.evaluate(
            """button => {
                const style = getComputedStyle(button, '::after');
                return {top: style.top, right: style.right, bottom: style.bottom, left: style.left, animation: style.animationName};
            }"""
        )
        assert spinner == {"top": "0px", "right": "0px", "bottom": "0px", "left": "0px", "animation": "spin"}
        submit.evaluate("button => button.removeAttribute('aria-busy')")

        page.locator("[name=approval_path]").select_option("skip")
        assert not page.locator("[name=citation]").is_visible()
        assert page.locator("[name=skip_reason]").is_visible()
        assert page.locator("[name=skip_reason]").get_attribute("required") == ""
        assert page.locator("#transition-modal").bounding_box()["width"] <= 360
        browser.close()


@pytest.mark.parametrize("entity_type", ["problems", "features"])
def test_explore_loads_refinement_preview_with_current_item_and_lineage(entity_type: str) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 390, "height": 844})
        page.goto(page_url)
        page.evaluate(
            """entityType => {
                window.fetch = url => String(url).endsWith('/refinement-context')
                    ? Promise.resolve({
                        ok: true, status: 200,
                        json: async () => ({has_context: true, entries: [
                            {label: 'Current item', text: 'Make AI work requests queueable'},
                            {label: 'Earlier Capture discussion', text: 'The team needs a visible request queue.'}
                        ]}),
                        text: async () => ''
                    })
                    : Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                openChat(entityType, 'item-id');
            }""",
            entity_type,
        )

        preview = page.locator("#explore-refinement-preview")
        preview.get_by_text("Make AI work requests queueable").wait_for(timeout=1000)
        assert preview.locator("header > small").inner_text() == "EXPLORE WORKSPACE"
        assert page.locator("#explore-preview-status").inner_text() == "LIVE CONTEXT"
        assert preview.get_by_text("Earlier Capture discussion").is_visible()
        assert page.locator("#chat-log article").first.locator("small").inner_text() == "LLM WIKI"
        assert page.locator(".quick-options").first.is_visible()
        modal = page.locator("#chat-modal").bounding_box()
        preview_box = preview.bounding_box()
        chat_box = page.locator("#chat-column").bounding_box()
        assert preview_box["x"] >= modal["x"]
        assert preview_box["x"] + preview_box["width"] <= modal["x"] + modal["width"]
        assert preview_box["y"] + preview_box["height"] <= chat_box["y"]
        browser.close()


@pytest.mark.parametrize("viewport", [{"width": 1280, "height": 900}, {"width": 390, "height": 844}])
def test_solution_card_opens_saved_detail_and_work_in_explore_workspace(viewport: dict[str, int]) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport=viewport)
        page.goto(page_url)
        page.evaluate(
            """() => {
                const solution = {id: 'saved-solution', problem_id: 'problem-1', title: 'Saved Solution title', outcome: 'A durable saved outcome', non_goals: 'No replacement modal', validation_criteria: '- [ ] Detail appears', state: 'approved', conflict_state: 'clear', created_at: '2026-08-21 10:00:00', category: 'General'};
                const problem = {id: 'problem-1', statement: 'The source Problem', detail: '', state: 'approved', category: 'General'};
                window.workbenchBoard = {captures: [], problems: [problem], features: [solution], tasks: []};
                window.boardItems = {'features:saved-solution': solution, 'problems:problem-1': problem};
                document.querySelector('#board').innerHTML = renderSwimlane('General', window.workbenchBoard);
                window.fetch = url => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) return Promise.resolve({ok: true, status: 200, json: async () => ({
                        has_context: false, entries: [], refinement_draft: null,
                        current_detail: {kind: 'solution', title: solution.title, outcome: solution.outcome, non_goals: solution.non_goals, validation_criteria: solution.validation_criteria, state: solution.state, conflict_state: solution.conflict_state, problem_statement: problem.statement}
                    }), text: async () => ''});
                    if (value.endsWith('/progress')) return Promise.resolve({ok: true, status: 200, json: async () => ({entries: [{id: 'entry-1', body: 'Implemented the important path', created_at: '2026-08-21 10:30:00', comments: []}], checklist: [{id: 'check-1', body: 'Detail appears', checked: 1}]}), text: async () => ''});
                    return Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                };
            }"""
        )
        page.get_by_text("Saved Solution title", exact=True).click()

        page.locator("#explore-preview-detail").get_by_text("A durable saved outcome", exact=True).wait_for(
            timeout=1000
        )
        assert page.locator("#chat-modal").evaluate("dialog => dialog.open")
        assert not page.locator("#item-detail-modal").evaluate("dialog => dialog.open")
        assert page.locator("#preview-detail-tab").get_attribute("aria-selected") == "true"
        assert page.locator("#explore-preview-status").inner_text() == "CURRENT"
        assert page.get_by_text("SAVED SOLUTION DETAIL").is_visible()
        assert page.locator("#apply-refinement-preview").count() == 0
        assert page.locator("#preview-work-tab").is_visible()
        page.locator("#preview-work-tab").click()
        page.get_by_text("Implemented the important path").wait_for(timeout=1000)
        assert page.locator('[data-check-text="check-1"]').input_value() == "Detail appears"
        modal = page.locator("#chat-modal").bounding_box()
        preview = page.locator("#explore-refinement-preview").bounding_box()
        assert preview["x"] >= modal["x"]
        assert preview["x"] + preview["width"] <= modal["x"] + modal["width"] + 1
        assert preview["y"] + preview["height"] <= modal["y"] + modal["height"] + 1
        browser.close()


@pytest.mark.parametrize("viewport", [{"width": 1280, "height": 900}, {"width": 390, "height": 844}])
def test_in_progress_card_opens_explore_with_work_selected(viewport: dict[str, int]) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport=viewport)
        page.goto(page_url)
        page.evaluate(
            """() => {
                const solution = {id: 'in-progress-1', problem_id: 'problem-1', title: 'Active work first', outcome: 'Resume the work without another click.', non_goals: '', validation_criteria: '- [ ] Work opens first', state: 'approved', conflict_state: 'clear', created_at: '2026-08-21 10:00:00', category: 'General'};
                const problem = {id: 'problem-1', statement: 'Resume active work quickly', detail: '', state: 'approved', category: 'General'};
                const board = {captures: [], problems: [problem], features: [solution], tasks: []};
                window.workbenchBoard = board;
                window.boardItems = {'features:in-progress-1': solution, 'problems:problem-1': problem};
                window.fetch = url => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) return Promise.resolve({ok: true, status: 200, json: async () => ({entries: [], refinement_draft: null, current_detail: {kind: 'solution', title: solution.title, outcome: solution.outcome, non_goals: solution.non_goals, validation_criteria: solution.validation_criteria, state: solution.state, conflict_state: solution.conflict_state, problem_statement: problem.statement}}), text: async () => ''});
                    if (value.endsWith('/progress')) return Promise.resolve({ok: true, status: 200, json: async () => ({entries: [{id: 'entry-1', body: 'Continue from this work log', created_at: '2026-08-21 10:30:00', comments: []}], checklist: [{id: 'check-1', body: 'Work opens first', checked: 0}]}), text: async () => ''});
                    return Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                };
                renderInProgress(board);
            }"""
        )

        page.locator('[data-progress-id="in-progress-1"] strong').click()
        page.get_by_text("Continue from this work log").wait_for(timeout=1000)
        assert page.locator("#preview-work-tab").get_attribute("aria-selected") == "true"
        assert page.locator("#explore-preview-status").inner_text() == "WORK"
        assert page.locator("#explore-preview-work").is_visible()
        assert page.locator("#preview-detail-tab").is_enabled()
        page.locator("#preview-detail-tab").click()
        assert (
            page.locator("#explore-preview-detail")
            .get_by_text("Resume the work without another click.", exact=True)
            .is_visible()
        )
        browser.close()


@pytest.mark.parametrize("viewport", [{"width": 1280, "height": 900}, {"width": 390, "height": 844}])
def test_capture_click_previews_problem_without_promoting_until_apply(viewport: dict[str, int]) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport=viewport)
        page.goto(page_url)
        page.evaluate(
            """() => {
                const capture = {id: 'capture-1', text: 'Raw idea stays in Capture while we refine it', created_at: '2026-08-21 10:00:00', category: 'General'};
                const board = {captures: [capture], problems: [], features: [], tasks: []};
                window.workbenchBoard = board;
                window.boardItems = {'captures:capture-1': capture};
                document.querySelector('#board').innerHTML = renderSwimlane('General', board);
                window.__promotions = [];
                window.__promotionAttempts = 0;
                window.fetch = (url, options = {}) => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) return Promise.resolve({ok: true, status: 200, json: async () => ({entries: [{label: 'Current item', text: capture.text}], next_draft: null, refinement_draft: null, current_detail: {kind: 'capture', title: capture.text, detail: capture.text, state: 'captured'}}), text: async () => ''});
                    if (value.endsWith('/next-chat')) return Promise.resolve(new Response('data: Let’s shape the Problem before moving it.\\n\\nevent: done\\ndata: done\\n\\n', {status: 200}));
                    if (value.endsWith('/draft')) return Promise.resolve({ok: true, status: 202, json: async () => ({id: 'problem-draft-job'}), text: async () => ''});
                    if (value.endsWith('/jobs/problem-draft-job')) return Promise.resolve({ok: true, status: 200, json: async () => ({status: 'completed'}), text: async () => ''});
                    if (value.endsWith('/jobs/problem-draft-job/result')) return Promise.resolve({ok: true, status: 200, json: async () => ({result: {title: 'Refinement should not change workflow state', detail: '## Context\\nThe raw idea needs a clear Problem.\\n\\n## Desired outcome\\nThe user reviews it before promotion.'}}), text: async () => ''});
                    if (value.endsWith('/promote') && options.method === 'POST') {
                        window.__promotionAttempts += 1;
                        if (window.__promotionAttempts === 1) return Promise.resolve({ok: false, status: 500, text: async () => 'Promotion failed'});
                        window.__promotions.push(JSON.parse(options.body));
                        return Promise.resolve({ok: true, status: 201, json: async () => ({id: 'problem-1', statement: 'Refinement should not change workflow state'}), text: async () => ''});
                    }
                    if (value.endsWith('/api/board')) return Promise.resolve({ok: true, status: 200, json: async () => ({captures: [], problems: [], features: [], tasks: []}), text: async () => ''});
                    if (value.includes('/recent-archive')) return Promise.resolve({ok: true, status: 200, json: async () => ({documents: []}), text: async () => ''});
                    if (value.includes('/completed-solutions')) return Promise.resolve({ok: true, status: 200, json: async () => ({solutions: []}), text: async () => ''});
                    return Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                };
            }"""
        )

        page.get_by_text("Raw idea stays in Capture while we refine it", exact=True).click()
        page.locator("#chat-modal[open]").wait_for(timeout=1000)
        assert page.locator("#chat-title").inner_text() == "Explore Problem"
        assert page.locator("#explore-refinement-preview").is_visible()
        assert page.get_by_text("CAPTURE CONTEXT").is_visible()
        assert page.evaluate("window.__promotions.length") == 0
        assert page.locator('[data-item-type="captures"]').count() == 1

        page.locator("#chat-message").fill("Clarify the underlying Problem")
        page.locator("#chat-form").evaluate("form => form.requestSubmit()")
        page.get_by_text("PROPOSED PROBLEM DETAIL").wait_for(timeout=3000)
        assert page.locator("#explore-preview-status").inner_text() == "PROBLEM READY"
        create = page.get_by_role("button", name="Create Problem")
        assert create.is_visible()
        assert page.evaluate("window.__promotions.length") == 0

        create.click()
        page.locator('#preview-job-status[data-state="error"]').wait_for(timeout=3000)
        assert page.locator("#chat-modal").evaluate("dialog => dialog.open")
        assert create.is_enabled()
        create.click()
        page.wait_for_function("window.__promotions.length === 1")
        assert page.evaluate("window.__promotions[0]") == {
            "statement": "Refinement should not change workflow state",
            "detail": "## Context\nThe raw idea needs a clear Problem.\n\n## Desired outcome\nThe user reviews it before promotion.",
        }
        assert not page.locator("#chat-modal").evaluate("dialog => dialog.open")
        browser.close()


@pytest.mark.parametrize("viewport", [{"width": 1280, "height": 900}, {"width": 390, "height": 844}])
def test_completed_solution_uses_read_only_explore_workspace(viewport: dict[str, int]) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport=viewport)
        page.goto(page_url)
        page.evaluate(
            """() => {
                const solution = {id: 'completed-1', problem_id: 'problem-1', problem_statement: 'A completed source Problem', title: 'Completed Solution title', outcome: 'The outcome is now durable.', non_goals: 'Do not mutate history.', completion_report: 'Human verified the result.', completion_evidence: 'All criteria passed.', completion_playbook_path: '2026/90. Archive/completed.md', archive_status: 'available'};
                window.completedSolutions = {'completed-1': solution};
                window.fetch = (url, options = {}) => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) return Promise.resolve({ok: true, status: 200, json: async () => ({entries: [{label: 'Earlier decision', text: 'Keep the final record immutable.'}]}), text: async () => ''});
                    if (value.endsWith('/progress')) return Promise.resolve({ok: true, status: 200, json: async () => ({entries: [{id: 'entry-1', body: 'Final evidence captured', created_at: '2026-08-21 10:30:00', comments: []}], checklist: [{id: 'check-1', body: 'All criteria passed', checked: 1}]}), text: async () => ''});
                    if (value.endsWith('/follow-up-problem') && options.method === 'POST') { window.__followUpRequested = true; return Promise.resolve({ok: true, status: 201, json: async () => ({id: 'follow-up-1'}), text: async () => ''}); }
                    return Promise.resolve({ok: true, status: 200, json: async () => ({captures: [], problems: [], features: [], tasks: []}), text: async () => ''});
                };
                openCompletedDetail('completed-1');
            }"""
        )

        page.get_by_text("The outcome is now durable.", exact=True).wait_for(timeout=1000)
        assert page.locator("#chat-modal").evaluate("dialog => dialog.open")
        assert not page.locator("#item-detail-modal").evaluate("dialog => dialog.open")
        assert page.locator("#preview-detail-tab").inner_text() == "Result"
        assert page.locator("#preview-context-tab").inner_text() == "Lineage"
        assert page.locator("#preview-work-tab").inner_text() == "Evidence"
        assert page.locator("#preview-archive-tab").inner_text() == "Archive"
        assert page.locator("#apply-refinement-preview").count() == 0
        assert page.locator("#progress-composer").count() == 0
        assert page.locator("#create-follow-up-problem").count() == 1
        page.locator("#preview-context-tab").click()
        assert page.get_by_text("Keep the final record immutable.").is_visible()
        page.locator("#preview-work-tab").click()
        assert page.get_by_text("Final evidence captured").is_visible()
        assert page.get_by_text("All criteria passed", exact=True).is_visible()
        page.locator("#preview-archive-tab").click()
        assert page.get_by_text("2026/90. Archive/completed.md").is_visible()
        modal = page.locator("#chat-modal").bounding_box()
        assert modal["width"] <= viewport["width"] - (18 if viewport["width"] <= 780 else 32) + 1
        assert modal["height"] <= viewport["height"] - (18 if viewport["width"] <= 780 else 32) + 1
        browser.close()


@pytest.mark.parametrize("viewport", [{"width": 1280, "height": 900}, {"width": 390, "height": 844}])
def test_chat_updates_refinement_in_preview_background_and_applies_without_review_modal(
    viewport: dict[str, int],
) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport=viewport)
        page.goto(page_url)
        page.evaluate(
            """() => {
                window.__contextReads = 0;
                window.__applied = null;
                window.fetch = (url, options = {}) => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) {
                        window.__contextReads += 1;
                        const updated = window.__contextReads > 1;
                        return Promise.resolve({ok: true, status: 200, json: async () => ({
                            has_context: true,
                            entries: [
                                {label: 'Current item', text: 'Make background refinement visible'},
                                {label: 'Recent discussion', text: updated ? 'The user chose a background draft.' : 'Keep context visible.'}
                            ],
                            focus: [{key: 'dependencies', label: 'Dependencies', status: 'weak'}],
                            refinement_draft: null
                        }), text: async () => ''});
                    }
                    if (value.endsWith('/chat')) return Promise.resolve(new Response(
                        'data: ✅ Ready. Your AI refinement is ready to review.\\n\\nevent: done\\ndata: done\\n\\n',
                        {status: 200}
                    ));
                    if (value.endsWith('/refine')) {
                        window.__backgroundRefinementReady = false;
                        window.__resolveBackgroundRefinement = () => { window.__backgroundRefinementReady = true; };
                        return Promise.resolve({ok: true, status: 202, json: async () => ({id: 'refinement-job'}), text: async () => ''});
                    }
                    if (value.endsWith('/jobs/refinement-job/result')) return Promise.resolve({ok: true, status: 200, json: async () => ({result: {title: 'Background refinement Preview', detail: '## Context\\nThe chat stays usable.\\n\\n## Dependencies\\nThe current context endpoint remains available.'}}), text: async () => ''});
                    if (value.endsWith('/jobs/refinement-job')) return Promise.resolve({ok: true, status: 200, json: async () => ({status: window.__backgroundRefinementReady ? 'completed' : 'running', progress: {completed: 0, total: 1}}), text: async () => ''});
                    if (value.includes('/items/')) {
                        window.__applied = JSON.parse(options.body);
                        return Promise.resolve({ok: true, status: 204, json: async () => null, text: async () => ''});
                    }
                    return Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                };
                openChat('features', 'solution-id');
            }"""
        )

        page.get_by_text("Keep context visible.").wait_for(timeout=1000)
        page.locator("#chat-message").fill("Keep the work in this Preview")
        page.locator("#chat-form").evaluate("form => form.requestSubmit()")
        page.locator('#preview-job-status[data-state="working"]').wait_for(timeout=3000)
        assert page.locator("#preview-job-status").inner_text() == "↻"
        assert "background" in page.locator("#preview-job-status").get_attribute("data-tooltip").lower()
        assert page.locator(".draft-generation .thinking").is_visible()
        assert page.get_by_text("Generating draft…").is_visible()
        assert page.get_by_text("✅ Ready. Your AI refinement is ready to review.").count() == 0
        assert page.locator("#preview-context-tab").get_attribute("aria-selected") == "true"
        assert page.get_by_text("The user chose a background draft.").is_visible()
        page.wait_for_function("!document.querySelector('#chat-form .primary').disabled")
        assert page.locator("#chat-form .primary").is_enabled()
        assert page.locator("#preview-job-status").get_attribute("data-state") == "working"
        assert page.get_by_text("Apply AI refinement").count() == 0
        assert not page.locator("#draft-modal").is_visible()

        page.evaluate("window.__resolveBackgroundRefinement()")
        page.locator('#preview-job-status[data-state="success"]').wait_for(timeout=3000)
        assert page.locator("#preview-detail-tab").get_attribute("aria-selected") == "true"
        assert page.locator("#explore-preview-status").inner_text() == "DRAFT READY"
        assert page.get_by_text("Background refinement Preview", exact=True).first.is_visible()
        assert page.get_by_text("The chat stays usable.").is_visible()
        assert page.locator(".draft-generation").count() == 0
        assert page.get_by_text("✅ Ready. Your AI refinement is ready to review.").is_visible()
        apply = page.locator("#apply-refinement-preview")
        assert apply.inner_text() == "Apply Refinement"

        page.locator("#preview-context-tab").click()
        assert page.locator("#explore-preview-status").inner_text() == "LIVE CONTEXT"
        assert page.get_by_text("The user chose a background draft.").is_visible()
        page.locator("#preview-detail-tab").click()
        assert page.locator("#explore-preview-status").inner_text() == "DRAFT READY"
        apply.click()
        page.get_by_text("Refinement applied").wait_for(timeout=3000)
        assert page.locator("#explore-preview-status").inner_text() == "APPLIED"
        assert page.evaluate("window.__applied") == {
            "title": "Background refinement Preview",
            "detail": "## Context\nThe chat stays usable.\n\n## Dependencies\nThe current context endpoint remains available.",
        }
        assert not page.locator("#draft-modal").is_visible()
        browser.close()


@pytest.mark.parametrize("viewport", [{"width": 1280, "height": 900}, {"width": 390, "height": 844}])
def test_explore_next_solution_uses_live_preview_and_creates_from_detail(viewport: dict[str, int]) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport=viewport)
        page.goto(page_url)
        page.evaluate(
            """() => {
                window.__createdSolution = null;
                window.fetch = (url, options = {}) => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) return Promise.resolve({ok: true, status: 200, json: async () => ({
                        has_context: true,
                        entries: [
                            {label: 'Current item', text: 'Make handoffs understandable'},
                            {label: 'Recent discussion', text: 'Keep the user in one workspace.'}
                        ],
                        next_draft: null,
                        refinement_draft: null
                    }), text: async () => ''});
                    if (value.endsWith('/next-chat')) return Promise.resolve(new Response(
                        'data: I will shape that outcome into a Solution.\\n\\nevent: done\\ndata: done\\n\\n',
                        {status: 200}
                    ));
                    if (value.endsWith('/draft')) {
                        window.__solutionDraftReady = false;
                        window.__resolveSolutionDraft = () => { window.__solutionDraftReady = true; };
                        return Promise.resolve({ok: true, status: 202, json: async () => ({id: 'solution-draft-job'}), text: async () => ''});
                    }
                    if (value.endsWith('/jobs/solution-draft-job/result')) return Promise.resolve({ok: true, status: 200, json: async () => ({result: {title: 'Keep Explore next in one workspace', outcome: 'The proposed Solution updates beside the conversation.', non_goals: 'Do not create the Solution automatically.', validation_criteria: '- [ ] Context and Detail remain available'}}), text: async () => ''});
                    if (value.endsWith('/jobs/solution-draft-job')) return Promise.resolve({ok: true, status: 200, json: async () => ({status: window.__solutionDraftReady ? 'completed' : 'running', progress: {completed: 0, total: 1}}), text: async () => ''});
                    if (value.endsWith('/features') && options.method === 'POST') return new Promise(resolve => {
                        window.__createdSolution = JSON.parse(options.body);
                        window.__resolveSolutionCreation = () => resolve({ok: true, status: 201, json: async () => ({id: 'solution-id'}), text: async () => ''});
                    });
                    return Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                };
                openNextChat('problems', 'problem-id');
            }"""
        )

        preview = page.locator("#explore-refinement-preview")
        page.get_by_text("Make handoffs understandable").wait_for(timeout=1000)
        assert preview.is_visible()
        assert page.locator("#chat-modal").get_attribute("class").find("with-preview") >= 0
        assert page.locator("#preview-context-tab").get_attribute("aria-selected") == "true"
        assert page.get_by_text("Review Solution proposal").count() == 0

        page.locator("#chat-message").fill("Make the Preview update automatically")
        page.locator("#chat-form").evaluate("form => form.requestSubmit()")
        page.locator('#preview-job-status[data-state="working"]').wait_for(timeout=3000)
        assert page.locator("#explore-preview-status").inner_text() == "DRAFTING…"
        assert "background" in page.locator("#preview-job-status").get_attribute("data-tooltip").lower()
        assert page.get_by_text("Keep the user in one workspace.").is_visible()
        page.wait_for_function("!document.querySelector('#chat-form .primary').disabled")

        page.evaluate("window.__resolveSolutionDraft()")
        page.locator('#preview-job-status[data-state="success"]').wait_for(timeout=3000)
        assert page.locator("#preview-detail-tab").get_attribute("aria-selected") == "true"
        assert page.locator("#explore-preview-status").inner_text() == "SOLUTION READY"
        assert page.get_by_text("PROPOSED SOLUTION DETAIL").is_visible()
        for label in ("Solution title", "Intended outcome", "Non-goals", "Validation criteria"):
            assert preview.get_by_text(label, exact=True).is_visible()
        create = page.locator("#apply-refinement-preview")
        assert create.inner_text() == "Create Solution"
        assert not page.locator("#draft-modal").is_visible()

        page.locator("#preview-context-tab").click()
        assert page.get_by_text("Keep the user in one workspace.").is_visible()
        page.locator("#preview-detail-tab").click()
        modal = page.locator("#chat-modal").bounding_box()
        preview_box = preview.bounding_box()
        assert preview_box["x"] >= modal["x"]
        assert preview_box["x"] + preview_box["width"] <= modal["x"] + modal["width"] + 1
        assert preview_box["y"] + preview_box["height"] <= modal["y"] + modal["height"] + 1
        if viewport["width"] <= 780:
            preview.evaluate("node => node.scrollTop = node.scrollHeight")
            criterion_box = preview.get_by_text("- [ ] Context and Detail remain available", exact=True).bounding_box()
            action_box = create.bounding_box()
            assert criterion_box["y"] + criterion_box["height"] <= action_box["y"]
        create.click()
        page.wait_for_function("window.__resolveSolutionCreation !== undefined")
        assert page.locator("#chat-modal").evaluate("dialog => dialog.open")
        assert create.get_attribute("aria-busy") == "true"
        page.evaluate("window.__resolveSolutionCreation()")
        page.wait_for_function("!document.querySelector('#chat-modal').open")
        assert page.evaluate("window.__createdSolution") == {
            "title": "Keep Explore next in one workspace",
            "outcome": "The proposed Solution updates beside the conversation.",
            "non_goals": "Do not create the Solution automatically.",
            "validation_criteria": "- [ ] Context and Detail remain available",
        }
        browser.close()


def test_background_refinement_error_stays_in_preview_with_context_and_tooltip() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 390, "height": 844})
        page.goto(page_url)
        page.evaluate(
            """() => {
                window.fetch = url => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) return Promise.resolve({ok: true, status: 200, json: async () => ({
                        has_context: true, entries: [{label: 'Current context', text: 'Context remains readable.'}], refinement_draft: null
                    }), text: async () => ''});
                    if (value.endsWith('/chat')) return Promise.resolve(new Response('data: Continue.\\n\\nevent: done\\ndata: done\\n\\n', {status: 200}));
                    if (value.endsWith('/refine')) return Promise.resolve({ok: false, status: 502, text: async () => 'AI failed'});
                    return Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                };
                openChat('problems', 'problem-id');
            }"""
        )
        page.get_by_text("Context remains readable.").wait_for(timeout=1000)
        page.locator("#chat-message").fill("Continue refining")
        page.locator("#chat-form").evaluate("form => form.requestSubmit()")

        status = page.locator('#preview-job-status[data-state="error"]')
        status.wait_for(timeout=3000)
        assert status.inner_text() == "!"
        assert status.get_attribute("data-tooltip") == "Refinement preview is unavailable. Please try again."
        assert page.locator("#preview-context-tab").get_attribute("aria-selected") == "true"
        assert page.get_by_text("Context remains readable.").is_visible()
        assert not page.locator("#draft-modal").is_visible()
        status.hover()
        assert "Refinement preview is unavailable" in status.evaluate(
            "node => getComputedStyle(node, '::after').content"
        )
        browser.close()


@pytest.mark.parametrize("viewport", [{"width": 1280, "height": 900}, {"width": 390, "height": 844}])
def test_completed_lineage_is_traceable_and_does_not_overflow(viewport: dict[str, int]) -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport=viewport)
        page.goto(page_url)
        page.evaluate(
            """() => {
                const claim = (id, text, classification = 'observed', evidenceIds = ['evidence-' + id]) => ({
                    id, text, classification, confidence: classification === 'inferred' ? 'medium' : null,
                    current_author_type: classification === 'inferred' ? 'ai' : 'deterministic',
                    current_revision_id: 'revision-' + id, evidence_ids: evidenceIds
                });
                const claims = {
                    capture: claim('capture', 'Original feedback with an extremelylongtoken'.repeat(10)),
                    problem: claim('problem', 'Refined user problem and desired outcome'),
                    solution: claim('solution', 'Final Solution direction', 'decided'),
                    complete: claim('complete', 'Human reviewed completion evidence', 'decided'),
                    cp: claim('cp', 'Capture was promoted', 'decided', ['evidence-capture', 'evidence-problem']),
                    ps: claim('ps', 'Not explicitly recorded', 'decided', ['evidence-problem', 'evidence-solution']),
                    sc: claim('sc', 'Tests and human review supported completion', 'decided', ['evidence-solution', 'evidence-complete']),
                    inferred: claim('inferred', 'Likely rationale from linked records', 'inferred'),
                    conflict: claim('conflict', 'The original request was modified', 'decided')
                };
                const evidence = Object.fromEntries(Object.keys(claims).map(id => ['evidence-' + id, {
                    id:'evidence-' + id, source_type:id, source_id:id + '-record', field_name:'detail', source_hash:'hash-' + id
                }]));
                const lineage = {
                    snapshot_id: 'snapshot-1', version: 1, claims, evidence,
                    lineage: {
                        stages: [
                            {kind:'capture', title:'Capture', record_type:'captures', record_id:'capture-1', claim_id:'capture', occurred_at:'2026-08-18 09:00:00', live_available:true},
                            {kind:'problem', title:'Problem', record_type:'problems', record_id:'problem-1', claim_id:'problem', occurred_at:'2026-08-19 10:00:00', live_available:true},
                            {kind:'solution', title:'Solution', record_type:'features', record_id:'solution-1', claim_id:'solution', occurred_at:'2026-08-20 11:00:00', live_available:true},
                            {kind:'complete', title:'Complete', record_id:'complete-1', claim_id:'complete', occurred_at:'2026-08-21 12:00:00', live_available:true}
                        ],
                        transitions: [
                            {claim_id:'cp',context_kind:'recorded_change'},
                            {claim_id:'ps',context_kind:'recorded_change',material_conflict:{status:'addressed', disposition:'modified'}},
                            {claim_id:'sc',context_kind:'decision_basis'}
                        ]
                    },
                    decision_changes:[{claim_id:'inferred',event_type:'ai_inferred'}],
                    conflicts:[{claim_id:'conflict',status:'addressed',basis:'explicit_decision',disposition:'modified'}]
                };
                window.completedSolutions = {'solution-1': {
                    id:'solution-1', problem_id:'problem-1', title:'Completed lineage', outcome:'Trace every decision',
                    completion_report:'Verified', completion_evidence:'Browser evidence', non_goals:'None', archive_status:'missing'
                }};
                window.fetch = (url) => {
                    const value = String(url);
                    let payload = {};
                    if (value.endsWith('/lineage')) payload = lineage;
                    else if (value.includes('/lineage/evidence/')) payload = {source_type:'capture',field_name:'text',excerpt:'Original source excerpt',captured_at:'2026-08-21 12:00:00',live_record:{available:false}};
                    else if (value.endsWith('/refinement-context')) payload = {entries:[]};
                    else if (value.endsWith('/progress')) payload = {checklist:[],entries:[]};
                    else if (value.endsWith('/items/captures/capture-1')) payload = {kind:'capture',title:'Original feedback',detail:'Original feedback',created_at:'2026-08-18 09:00:00'};
                    else if (value.endsWith('/items/problems/problem-1')) payload = {kind:'problem',title:'Refined user problem',detail:'Desired outcome is traceability',state:'completed',created_at:'2026-08-19 10:00:00'};
                    else if (value.endsWith('/items/features/solution-1')) payload = {kind:'solution',title:'Final Solution direction',problem_statement:'Refined user problem',outcome:'Trace every decision',state:'approved',created_at:'2026-08-20 11:00:00'};
                    return Promise.resolve({ok:true,status:200,json:async()=>payload,text:async()=>''});
                };
                openCompletedWorkspace('solution-1');
            }"""
        )

        page.locator(".lineage-view").wait_for(state="attached", timeout=2000)
        page.locator("#preview-context-tab").click()
        assert page.locator(".lineage-stage").count() == 4
        assert page.locator(".lineage-stage[data-lineage-record]").count() == 3
        assert page.locator(".lineage-time").count() == 4
        assert page.get_by_text("v1", exact=True).count() == 0
        assert page.get_by_text("Open record", exact=False).count() == 0
        assert page.get_by_text("Source 1", exact=True).count() == 0
        assert page.get_by_text("Recorded change", exact=True).count() == 2
        assert page.get_by_text("Not explicitly recorded", exact=True).count() == 0
        assert page.locator('[data-lineage-evidence="evidence-capture"][data-reference-number="1"]').count() == 2
        assert page.locator('[data-lineage-evidence="evidence-problem"][data-reference-number="2"]').count() == 3
        assert page.get_by_text("AI inferred · medium confidence", exact=True).count() == 1
        assert page.get_by_text("Conflict addressed · modified", exact=True).is_visible()
        assert page.locator("#explore-preview-content").evaluate("node => node.scrollWidth <= node.clientWidth + 1")
        assert page.locator("#chat-log").evaluate("node => node.scrollWidth <= node.clientWidth + 1")

        page.locator('[data-record-type="problems"]').click()
        page.get_by_text("Desired outcome is traceability", exact=True).wait_for(timeout=1000)
        page.locator("#item-detail-modal").evaluate("dialog => dialog.close()")
        page.locator('.lineage-reference-chip[data-reference-number="1"]').first.click()
        page.get_by_text("Original source excerpt", exact=True).wait_for(timeout=1000)
        assert page.get_by_text("Preserved snapshot", exact=True).is_visible()
        assert page.locator(".lineage-stage").first.locator(".lineage-reference-popover").count() == 1
        browser.close()


def test_latest_background_refinement_reopens_in_detail_with_context_tab_available() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.goto(page_url)
        page.evaluate(
            """() => {
                window.fetch = url => String(url).endsWith('/refinement-context')
                    ? Promise.resolve({ok: true, status: 200, json: async () => ({
                        has_context: true,
                        entries: [{label: 'Earlier discussion', text: 'Preserve this context.'}],
                        refinement_draft: {title: 'Restored detail draft', detail: '## Context\\nRestored after reopening.', applied: false}
                    }), text: async () => ''})
                    : Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                openChat('problems', 'problem-id');
            }"""
        )
        page.get_by_text("Restored detail draft", exact=True).first.wait_for(timeout=1000)
        assert page.locator("#preview-detail-tab").get_attribute("aria-selected") == "true"
        assert page.locator("#apply-refinement-preview").is_visible()
        page.locator("#preview-context-tab").click()
        assert page.get_by_text("Preserve this context.").is_visible()
        assert not page.locator("#draft-modal").is_visible()
        browser.close()


def test_stale_background_refinement_cannot_replace_a_new_item_preview() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        page.evaluate(
            """() => {
                window.fetch = (url, options = {}) => {
                    const value = String(url);
                    if (value.endsWith('/refinement-context')) return Promise.resolve({ok: true, status: 200, json: async () => ({
                        has_context: true, entries: [{label: 'Current item', text: value.includes('first-id') ? 'First item' : 'Second item'}], refinement_draft: null
                    }), text: async () => ''});
                    if (value.endsWith('/chat')) return Promise.resolve(new Response('data: Continue.\\n\\nevent: done\\ndata: done\\n\\n', {status: 200}));
                    if (value.includes('first-id/refine')) return new Promise(resolve => {
                        options.signal?.addEventListener('abort', () => {});
                        window.__resolveStale = () => resolve({ok: true, status: 200, json: async () => ({title: 'Stale draft', detail: 'Wrong item'}), text: async () => ''});
                    });
                    return Promise.resolve({ok: true, status: 200, json: async () => ({}), text: async () => ''});
                };
                openChat('problems', 'first-id');
            }"""
        )
        page.get_by_text("First item").wait_for(timeout=1000)
        page.locator("#chat-message").fill("Refine first")
        page.locator("#chat-form").evaluate("form => form.requestSubmit()")
        page.locator('#preview-job-status[data-state="working"]').wait_for(timeout=3000)
        page.evaluate("openChat('features', 'second-id')")
        page.get_by_text("Second item").wait_for(timeout=1000)
        page.evaluate("window.__resolveStale()")
        page.wait_for_timeout(100)
        assert page.get_by_text("Stale draft").count() == 0
        assert page.locator("#preview-detail-tab").is_disabled()
        assert page.locator("#chat-title").inner_text() == "Explore this Solution"
        browser.close()


def test_switching_locale_refreshes_the_open_solution_work_log_summary() -> None:
    from playwright.sync_api import sync_playwright

    page_url = (Path(__file__).parents[1] / "llm_wiki" / "static" / "index.html").as_uri()
    with sync_playwright() as driver:
        try:
            browser = driver.chromium.launch(headless=True)
        except Exception as error:
            pytest.skip(f"Playwright browser artifact unavailable locally: {error}")
        page = browser.new_page()
        page.goto(page_url)
        page.evaluate(
            """async () => {
                window.boardItems = {'features:solution-1': {
                    id:'solution-1', problem_id:'problem-1', title:'Stored summary', outcome:'Outcome',
                    non_goals:'None', state:'approved', conflict_state:'clear', created_at:'2026-08-21 12:00:00'
                }};
                window.workbenchBoard = {captures:[],problems:[{id:'problem-1',statement:'Problem'}],features:[window.boardItems['features:solution-1']]};
                chatTarget = {type:'features',id:'solution-1',mode:'current'};
                if (!chatModal.open) chatModal.showModal();
                window.fetch = (url, options = {}) => {
                    const value=String(url), headers=options.headers||{}, locale=headers['X-LLM-Wiki-Locale']||activeLocale;
                    let payload={};
                    if(value.endsWith('/settings/locale')) payload={locale,explicit:true,supported_locales:['ko','en']};
                    else if(value.endsWith('/board')) payload=window.workbenchBoard;
                    else if(value.endsWith('/dashboard')) payload={goals:[],events:[]};
                    else if(value.endsWith('/progress')) payload={entries:[{
                        id:'entry-1',body:'원문 작업 기록',image_data:'aGVsbG8=',image_media_type:'image/png',
                        image_summary:locale==='ko'?'한글 이미지 요약':'English image summary',created_at:'2026-08-21 12:00:00',comments:[]
                    }],checklist:[]};
                    return Promise.resolve({ok:true,status:200,json:async()=>payload,text:async()=>''});
                };
                await setLocale('ko');
            }"""
        )
        page.get_by_text("한글 이미지 요약", exact=True).wait_for(state="attached", timeout=1000)
        assert page.get_by_text("원문 작업 기록", exact=True).count() == 1
        browser.close()
