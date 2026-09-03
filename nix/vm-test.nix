# The NixOS module put through a Question Set in a VM.
#
# The round trip itself — a Set submitted, a Response posted back, the CLI
# printing it and exiting 0 — is already covered in-process by the crate tests.
# Doing it again here is only worth the VM for what wraps around it, which no
# in-process test can see: a unit that starts itself at boot, a Data Directory
# systemd creates and hands over, a database that outlives the process that made
# it, a server binary running from the store rather than a working tree, and the
# CLI on `PATH` with nothing set in the environment at all.
#
# Push delivery stays out of scope: it needs the browser vendors' push services,
# which a test VM has no route to. The server's own tests cover what gets sent.
{
  testers,
  git,
  # The flake's NixOS module, which closes over the flake's package — there is
  # no `pkgs.verkstead` for it to find by name.
  module,
  # What the VM runs: `verkstead-source`, deliberately, rather than the module's
  # own default. See where it is pinned below.
  package,
}:

let
  # Where the grilling half of this test works. Named once, up here, because the
  # probe, the machine's fixtures and the test script all say them — and a
  # sandbox built around a path one of them wrote down differently is one that
  # proves nothing.
  grillingRepo = "/srv/repos/grilling";
  account = "/srv/repos/account";

  # The service's home, which is the module's default. Nothing is read out of it
  # any more — the git identity and the gh login are settings files now — so what
  # it holds here is what has to stay outside a sandbox.
  home = "/var/lib/verkstead/home";

  # And the shared Rust build cache the module passes, which is what a session's
  # cargo and sccache write into. The module's own, said again here for the same
  # reason the home is: the probe looks for it by path.
  cacheDir = "/var/cache/verkstead";
in

testers.runNixOSTest {
  name = "verkstead-module";

  nodes.machine =
    {
      config,
      pkgs,
      ...
    }:
    let
      # What a session's sandbox looks like from inside, reported one
      # `key=value` line at a time.
      #
      # The same shape as the sandbox tests in `crates/server/tests/sandbox.rs`,
      # and for the same reason: a read-only bind and a directory nobody may
      # write look identical to `test -w`, so each path is attempted rather than
      # asked of its metadata.
      report = pkgs.writeShellScript "verkstead-sandbox-report" ''
        set -u

        say() { printf 'verkstead-probe %s=%s\n' "$1" "$2"; }

        dir() {
            if [ ! -d "$1" ]; then say "$2" absent; return; fi
            if (exec 3> "$1/.verkstead-probe") 2>/dev/null; then
                rm -f "$1/.verkstead-probe"
                say "$2" write
            elif ls "$1" >/dev/null 2>&1; then
                say "$2" read
            else
                say "$2" hidden
            fi
        }

        file() {
            if [ ! -f "$1" ]; then say "$2" absent; return; fi
            # Appending is the write, and it changes not a byte of what is
            # there. In a subshell, because a redirection that fails takes the
            # shell attempting it down with it.
            if (exec 3>> "$1") 2>/dev/null; then
                say "$2" write
            elif cat "$1" >/dev/null 2>&1; then
                say "$2" read
            else
                say "$2" hidden
            fi
        }

        worktree=$1

        dir "$worktree" worktree
        dir ${grillingRepo}/.git git-dir
        dir /srv/repos/inside sibling
        dir /nix nix

        # A bind's parents have to exist for it to land on, so the Repo is
        # inside as the scaffolding under its git directory — listed rather than
        # written to, because what is worth saying is what did *not* come in
        # with it.
        say repo-holds "$(ls -A ${grillingRepo} | sort | tr '\n' ' ')"

        dir "$HOME/.claude" claude-dir
        file "$HOME/.claude.json" claude-config
        file "$HOME/.gitconfig" gitconfig
        file "$HOME/.config/gh/hosts.yml" gh
        say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"

        # The shared Rust build cache, which on a module install is under
        # `/var/cache` rather than in the home above — so what a session's cargo
        # and sccache write goes somewhere the unit made and kept, and the
        # listing of HOME stays the two files it has always been.
        dir ${cacheDir} build-cache
        say cargo-home "''${CARGO_HOME-unset}"
        say wrapper "''${RUSTC_WRAPPER-unset}"
        say sccache-size "''${SCCACHE_CACHE_SIZE-unset}"
        # And the sccache the server resolved off this unit's own path, which is
        # the whole of what makes the compiling cached rather than only the
        # downloading.
        file "''${RUSTC_WRAPPER-/nowhere}" sccache

        # Out of the sandbox's own `/proc`, which is a fresh procfs in a pid
        # namespace of its own — so this says the mount happened as well as
        # saying what the hostname is.
        say hostname "$(cat /proc/sys/kernel/hostname)"
        say tmp-holds "$(ls -A /tmp | wc -l)"

        # The server the sandbox was started by, reached over the host's own
        # loopback. A network namespace of its own would find nothing there.
        if curl --silent --max-time 10 --output /dev/null \
            http://127.0.0.1:8422/api/v1/health; then
            say network reached
        else
            say network unreachable
        fi

        # What a session actually is. `claude` is node, and node is a JIT: it
        # aborts where it cannot make a page it wrote executable, which is a
        # hardening setting away either way.
        if node -e 'console.log("jit")' >/dev/null 2>&1; then
            say jit ran
        else
            say jit refused
        fi
      '';

      # And the sandbox itself, built around the Conversation the running
      # service made.
      #
      # The surface is `crates/server/src/sandbox.rs`'s to decide and its own
      # tests' to check. What no in-process test can ask is whether the packaged
      # unit permits one at all — the namespaces, the mounts, the `/proc` — so
      # that is what this runs under the service's hardening.
      probe = pkgs.writeShellScript "verkstead-sandbox-probe" ''
        set -eu

        # The worktree the running service made, found rather than named: which
        # directory it chose is the server's to decide, and a path written here
        # would be the test agreeing with itself about it.
        worktree=$(echo /var/lib/verkstead/worktrees/*)
        if [ ! -d "$worktree" ]; then
            echo "no worktree under the data directory" >&2
            exit 1
        fi

        # And the sccache a session compiles through, resolved off this unit's
        # own path rather than named by store path: that the module put one
        # within the service's reach is half of what the cache lines below ask,
        # and a path written here would be the test agreeing with itself again.
        if ! sccache=$(command -v sccache); then
            echo "no sccache on the unit's path" >&2
            exit 1
        fi

        exec bwrap \
            --die-with-parent \
            --unshare-all --share-net --hostname verkstead \
            --ro-bind /nix /nix \
            --ro-bind /bin /bin \
            --ro-bind /etc /etc \
            --ro-bind /run/current-system /run/current-system \
            --proc /proc --dev /dev --tmpfs /tmp \
            --dir "$HOME" \
            --bind "$worktree" "$worktree" \
            --bind ${grillingRepo}/.git ${grillingRepo}/.git \
            --bind ${account}/.claude "$HOME/.claude" \
            --bind ${account}/.claude.json "$HOME/.claude.json" \
            --bind ${cacheDir} ${cacheDir} \
            --ro-bind "$sccache" /verkstead/bin/sccache \
            --chdir "$worktree" \
            --setenv HOME "$HOME" \
            --setenv PATH /run/current-system/sw/bin:/nix/var/nix/profiles/default/bin:/usr/bin:/bin \
            --setenv CARGO_HOME ${cacheDir}/cargo \
            --setenv RUSTC_WRAPPER /verkstead/bin/sccache \
            --setenv SCCACHE_DIR ${cacheDir}/sccache \
            --setenv SCCACHE_CACHE_SIZE 30G \
            ${report} "$worktree"
      '';
    in
    {
      imports = [ module ];

      services.verkstead.enable = true;

      # Not an oversight, and not to be helpfully removed: the module defaults to
      # the released binary, and a test fed that would be exercising whatever the
      # last release contains rather than the tree it is run against — which makes
      # it worthless as a check on a branch. The pin is about *what* is tested,
      # not about network access: a `fetchurl` is a fixed-output derivation and
      # the binary would download here perfectly well.
      services.verkstead.package = package;

      # The module refuses to build without a Watched Path, and the service
      # refuses to start with one that is not there. Two of them, so that the
      # sandbox is exercised where it has real work to do: one under `/home`,
      # which the hardening replaces with an empty tmpfs and the module then binds
      # back through, and one outside it. The unit coming up at all is what says
      # both arrived.
      services.verkstead.watchedPaths = [
        "/srv/repos"
        "/home/watched"
      ];

      systemd.tmpfiles.rules = [
        "d /srv/repos 0755 root root -"
        "d /home/watched 0755 root root -"
        # The Agent Profile's pair, owned by the service because a session
        # writes its own session logs and settings into it. The repository a
        # Conversation is grilled about is deliberately not here: `committed`
        # makes that one as root and hands it over afterwards, since git refuses
        # to work in a repository belonging to somebody else whichever way round
        # the two are.
        "d ${account} 0755 verkstead verkstead -"
        "d ${account}/.claude 0755 verkstead verkstead -"
        "f ${account}/.claude.json 0644 verkstead verkstead - {}"
      ];

      # The CLI finds its own git through the package's wrapper; this one is here
      # so the test can build the repository the CLI then reads.
      #
      # curl and node are the two tools the sandbox probe runs *inside* a sandbox,
      # and they are on the system profile rather than named by store path because
      # that profile is what a session's `PATH` points at — so finding them there
      # is part of what is being asked.
      environment.systemPackages = [
        git
        pkgs.curl
        pkgs.nodejs
      ];

      # A sandbox under the packaged unit, run on demand rather than at boot.
      #
      # The hardening is not copied — it *is* `verkstead.service`'s, read straight
      # off the module's own output, so a setting that drifts here cannot drift
      # away from what the service runs under. Only what makes it a probe rather
      # than a server is replaced: what it runs, and that it runs once.
      systemd.services.verkstead-sandbox-probe = {
        description = "A sandbox started under Verkstead's own unit";

        # Including the service's `PATH`, so the bwrap the probe calls is the one
        # the module put within the server's reach and not one this test found.
        path = config.systemd.services.verkstead.path;

        serviceConfig = config.systemd.services.verkstead.serviceConfig // {
          ExecStart = "${probe}";
          Type = "oneshot";
          Restart = "no";
        };
      };

      # The fixtures, in the system closure rather than written from the test
      # script, so the YAML stays YAML and is not two layers of escaping deep.
      environment.etc = {
        "verkstead-vm-test/set.yaml".text = ''
          # A Question Set as an agent sends it. `project`, `branch` and `diff`
          # are absent on purpose: the CLI derives the first two from the
          # working directory, the server composes the third off the Worktree
          # the Set was asked from, and whatever a Set claims of any of them is
          # overwritten.

          title: Does the module hold up in a VM?

          preface: |
            Asked from inside the test VM, so that something has to travel the
            whole way: agent to server to human's device and back again.

          questions:
            - label: Q1
              text: Did the service come up on its own?
              options:
                - n: 1
                  text: It did, and its database is in the data directory.
                  recommended: true
                - n: 2
                  text: It did not.

            - label: Q2
              text: Anything else worth recording?
        '';

        # Two Responses of the same shape, distinguishable in the CLI's output, so
        # a test that answers two Sets can tell which answer reached which agent.
        #
        # JSON rather than YAML, because they go in the way the browser sends
        # one: the human answers in the viewer, and the viewer's own namespace
        # is what it posts to. The agents' half of the wire speaks YAML and is
        # exercised on the way *out* — the CLI submits its Set that way, and
        # reads its Response back that way.
        "verkstead-vm-test/first-response.json".text = builtins.toJSON {
          answers = [
            {
              label = "Q1";
              selected = 1;
            }
            {
              label = "Q2";
              free_text = "The first Set came back to the agent that asked it.";
            }
          ];
          comment = "Posted through the same API the web UI posts through.\n";
        };

        # A gitconfig and a gh login of the machine's own, neither of which a
        # session has any business seeing: who a commit is by and what `gh` is
        # logged in as are settings files in the Data Directory now, and reach a
        # session in its environment. Both are here so that "no credentials of
        # the host's inside" is a claim about files that exist rather than about
        # ones nobody made.
        "verkstead-vm-test/gitconfig".text = ''
          [user]
          	name = Whoever The Host Is
          	email = host@verkstead.invalid
        '';

        "verkstead-vm-test/gh-hosts.yml".text = ''
          github.com:
              user: nobody
        '';

        "verkstead-vm-test/second-response.json".text = builtins.toJSON {
          answers = [
            {
              label = "Q1";
              selected = 1;
            }
            {
              label = "Q2";
              free_text = "The second agent recovered its wait across the restart.";
            }
          ];
          comment = "Answered after the service had been stopped and started under it.\n";
        };
      };
    };

  testScript = ''
    import json
    import re
    import shlex

    # Where the agents' Sets are asked from. The name is distinctive so that
    # finding it in the human's page proves the CLI derived it from *this*
    # directory.
    REPO = "/root/vm-project"

    # Likewise distinctive, and for the same reason: "main" would match the
    # page's own `<main>` and prove nothing.
    BRANCH = "vm-branch"


    # The Conversation the agents here ask from, filled in once there is a Repo
    # to hang one off. Every Set is asked from one, and what says which is the
    # base URL a session is given — see `asking_from` below.
    ASKING_FROM = None


    def asking_from():
        """The base URL a session on the Conversation above is given, which is
        what the sandbox sets `VERKSTEAD_SERVER` to."""
        assert ASKING_FROM is not None, "no Conversation to ask from yet"
        return f"http://127.0.0.1:8422/conversations/{ASKING_FROM}"


    def ask(name):
        """Start `verkstead ask` the way an agent does — in the background, so the
        wait outlives the caller — and leave its stdout, stderr and exit status
        in a directory of its own.

        `VERKSTEAD_SERVER` is what a session's sandbox is given: the server,
        scoped to the Conversation it is asking from. Nothing else in the
        environment is set, and nothing about the Set says which Conversation it
        belongs to — the URL is the whole of the attribution.

        Every descriptor of the backgrounded subshell is redirected, the two the
        agent's own output goes to and the rest besides: anything it inherited
        would hold the test driver's pipe open, and the driver waits for that
        pipe to close before it moves on.
        """
        machine.succeed(f"mkdir -p /root/{name}")
        machine.succeed(
            f"( cd {REPO} && VERKSTEAD_SERVER={asking_from()}"
            " verkstead ask /etc/verkstead-vm-test/set.yaml"
            f" > /root/{name}/response.yaml 2> /root/{name}/log;"
            f" echo $? > /root/{name}/status )"
            " < /dev/null > /dev/null 2>&1 &"
        )


    def timeline():
        """The Conversation the agents here ask from, as the workbench reads it —
        which is where a Set arrives now that there is no list of them."""
        return json.loads(
            machine.succeed(
                "curl -sf"
                f" http://127.0.0.1:8422/api/ui/conversations/{ASKING_FROM}"
            )
        )["timeline"]


    def sets_on_timeline():
        """Every Question Set on that Timeline, oldest first."""
        return [
            event["QuestionSet"] for event in timeline() if "QuestionSet" in event
        ]


    def waiting(name):
        """The id of the Set the agent in `name` submitted, once the server has
        taken it.

        Asked of the server rather than of the agent: a wait that goes to plan
        is silent, so the CLI says nothing between submitting a Set and printing
        the Response. The Timeline of the Conversation it was asked from is
        where an arrival shows, and a Set that is neither answered nor locked
        reads there as `Waiting` — so with one agent asking at a time, the Set
        waiting there is that agent's.
        """
        machine.wait_until_succeeds(
            "curl -sf"
            f" http://127.0.0.1:8422/api/ui/conversations/{ASKING_FROM}"
            " | grep -q Waiting"
        )
        asked = sets_on_timeline()
        still_waiting = [
            found for found in asked if "Waiting" in found["standing"]
        ]
        assert len(still_waiting) == 1, (
            f"expected the one Set {name} asked to be waiting, got:\n{asked}"
        )

        # The agent has to still be there to be answered: a CLI that submitted
        # and then died would leave the Set waiting just the same.
        assert not machine.succeed(
            f"test -e /root/{name}/status && cat /root/{name}/status || true"
        ).strip(), f"{name} exited before its Set was answered"

        return still_waiting[0]["set_id"]


    def answer(set_id, fixture):
        """Answer a Set the way the human does: the viewer's own namespace, in
        JSON, which is the one route a browser posts a Response through from the
        workbench and from a phone alike.

        Not scoped to a Conversation, unlike everything the agents' half does: a
        Set is answered by whoever has its id, and which Conversation it belongs
        to is the server's to know. The outcome comes back in the body rather
        than as a status — every one of them is something the viewer has to say
        in words — so `curl -sf` proves nothing on its own and the answer is
        read.
        """
        outcome = machine.succeed(
            "curl -sf -X POST -H 'Content-Type: application/json'"
            f" --data-binary @/etc/verkstead-vm-test/{fixture}"
            f" http://127.0.0.1:8422/api/ui/sets/{set_id}/response"
        ).strip()
        assert outcome == '"Accepted"', f"the Response was answered {outcome}"


    def collect(name):
        """Wait for the agent in `name` to finish, insist it exited 0, and take
        what it printed."""
        machine.wait_until_succeeds(f"test -s /root/{name}/status")
        status = machine.succeed(f"cat /root/{name}/status").strip()
        if status != "0":
            log = machine.succeed(f"cat /root/{name}/log")
            raise AssertionError(f"the CLI exited {status}:\n{log}")
        return machine.succeed(f"cat /root/{name}/response.yaml")


    def status_code(url):
        return machine.succeed(
            f"curl -s -o /dev/null -w '%{{http_code}}' '{url}'"
        ).strip()


    start_all()

    with subtest("the service starts itself at boot"):
        # Nothing above this line started it. It is `wantedBy` multi-user.target,
        # so by the time the target is reached it is either running or the
        # module is wrong.
        machine.wait_for_unit("multi-user.target")
        machine.succeed("systemctl is-active --quiet verkstead.service")
        machine.wait_for_open_port(8422)
        machine.succeed("curl -sf http://127.0.0.1:8422/api/v1/health")

    with subtest("the database is in the data directory, owned by the service"):
        # The server opens the database before it binds, so the open port above
        # already says the file exists; what is asserted here is where it is and
        # whose it is.
        owner = machine.succeed("stat -c %U:%G /var/lib/verkstead/verkstead.db").strip()
        assert owner == "verkstead:verkstead", f"the database is owned by {owner}"

        directory = machine.succeed("stat -c %U:%G:%a /var/lib/verkstead").strip()
        assert directory == "verkstead:verkstead:750", f"the data directory is {directory}"

    with subtest("the server run from the store serves the viewer built into it"):
        # The viewer is inside the binary rather than beside it, so there is
        # nothing to point the server at and nothing that can be missing from the
        # package — but a build that embedded an empty directory would still start,
        # answer the API, and hand the phone a blank page. This is what catches
        # that, and nothing in-process can: the tests there stand up a fixture site
        # of their own precisely so they need no `pnpm build`.
        document = machine.succeed("curl -sf http://127.0.0.1:8422/")
        assert '<div id="app">' in document, f"no viewer at the root:\n{document}"

        # And the bundles it names are there under the names it names them by,
        # which is the half a document alone does not prove.
        bundles = re.findall(r'(?:href|src)="(/assets/[^"]+)"', document)
        assert bundles, f"the document names no bundle:\n{document}"
        for bundle in bundles:
            machine.succeed(f"curl -sf -o /dev/null http://127.0.0.1:8422{bundle}")

    with subtest("the update check is on by default, and a GitHub out of reach costs nothing"):
        # What the unit is running is readable in the unit, which is the point of
        # passing flags rather than setting the environment: with `updateCheck`
        # left alone, the opt-out is not among them.
        unit = machine.succeed("systemctl cat verkstead.service")
        assert "--no-update-check" not in unit, f"the check is off by default:\n{unit}"

        # And there is no route out of this VM, so the poll behind that check is
        # failing — which is the whole of what a failed poll is meant to cost.
        # The viewer is still answered, saying there is nothing to update to, and
        # the service is still up. The verdicts themselves are the server's own
        # tests' subject; what needs a VM is that a service reaching for GitHub
        # from inside this sandbox does not fall over.
        notice = machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/update").strip()
        assert notice == '"Current"', f"the Update Notice said {notice}"
        machine.succeed("systemctl is-active --quiet verkstead.service")

    def committed(path, branch=BRANCH, owner="verkstead:verkstead"):
        """A git repository at `path`, with one commit on `branch`, belonging to
        whoever will be working in it.

        The owner is not decoration. The service reads these repositories and
        writes them — `git worktree add` writes the repository's git directory,
        not just a checkout — and git refuses to read one it finds under another
        user at all, before Verkstead gets a say. So a Repo the service is given
        is one it has to own, and a fixture left as root's would be testing an
        arrangement nothing can work in.
        """
        machine.succeed(f"git -c init.defaultBranch={branch} init -q {path}")
        machine.succeed(f"echo committed > {path}/tracked.txt")
        machine.succeed(f"git -C {path} add -A")
        machine.succeed(
            f"git -C {path} -c user.name=Verkstead -c user.email=vm@verkstead.invalid"
            " -c commit.gpgsign=false commit -q -m init"
        )
        machine.succeed(f"chown -R {owner} {path}")


    def register(path):
        """Ask the server to take on the repository at `path`, and hand back the
        outcome it named."""
        return machine.succeed(
            "curl -sf -X POST -H 'Content-Type: application/json'"
            f" -d '{{\"path\":\"{path}\"}}'"
            " http://127.0.0.1:8422/api/ui/repos"
        ).strip()


    def post(path, body):
        """POST a JSON body to the viewer's own namespace, and hand back what
        came back — which is what the workbench would have received."""
        return machine.succeed(
            "curl -sf -X POST -H 'Content-Type: application/json'"
            f" -d {shlex.quote(json.dumps(body))}"
            f" http://127.0.0.1:8422{path}"
        ).strip()


    with subtest("a repo inside a watched path registers, and one outside cannot"):
        # Both watched paths, because they are exposed to the sandbox two
        # different ways: `/srv/repos` is somewhere the hardening leaves in
        # place, and `/home/watched` is under a directory it replaces with an
        # empty tmpfs and the module binds back through. A service that cannot
        # see the second would refuse it exactly as it refuses one outside.
        for watched in ["/srv/repos/inside", "/home/watched/inside"]:
            committed(watched)
            outcome = register(watched)
            assert outcome == '"Added"', f"{watched} was answered {outcome}"

        # And the boundary itself, from inside the running service rather than
        # from a unit test. `/srv/elsewhere` is somewhere the service can see
        # perfectly well and was not given, so what refuses it is the boundary
        # and not the sandbox — which is the half worth proving here.
        committed("/srv/elsewhere")
        outcome = register("/srv/elsewhere")
        assert outcome == '"OutsideWatchedPaths"', f"/srv/elsewhere was answered {outcome}"

        listed = machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/repos")
        assert "/srv/elsewhere" not in listed, f"a refused repo is on the list:\n{listed}"

    # Somewhere for the agents' Sets to land. Every Set is asked from a
    # Conversation, and the base URL a session is given is what says which — so a
    # test standing in for a session has to be given the same thing.
    #
    # Against a registered Repo, which is the only kind there is; the directory
    # the CLI derives `project` and `branch` from is the agent's own working
    # directory below, and deliberately not this one.
    repos = json.loads(machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/repos"))
    started = json.loads(post("/api/ui/conversations", {"repo_id": repos[0]["id"]}))
    ASKING_FROM = started["Started"]["id"]

    # The repository an agent always asks from, and which the CLI reads
    # `project` and `branch` out of by shelling out to git.
    machine.succeed(f"git -c init.defaultBranch={BRANCH} init -q {REPO}")
    machine.succeed(f"echo committed > {REPO}/tracked.txt")
    machine.succeed(f"git -C {REPO} add -A")
    machine.succeed(
        f"git -C {REPO} -c user.name=Verkstead -c user.email=vm@verkstead.invalid"
        " -c commit.gpgsign=false commit -q -m init"
    )

    with subtest("an agent's Set is answered through the API and printed by the CLI"):
        ask("first")
        first = waiting("first")

        # The Set surfaces on the Timeline of the Conversation it was asked
        # from, which is the whole of the attribution: nothing about the Set
        # says which Conversation it belongs to, only the base URL the session
        # was given. Asked of the API rather than of a page, because the page is
        # drawn in the browser and a `curl` of it is the empty document above.
        titles = [found["title"] for found in sets_on_timeline()]
        assert titles == ["Does the module hold up in a VM?"], (
            f"the Set is not on the Conversation's Timeline: {titles}"
        )

        # What the CLI derived from the working directory, rather than anything
        # the Set claimed, comes with the Set itself: the Timeline carries the
        # table of what was asked, and the document is what the pane showing it
        # fetches.
        #
        # The Diff is not among them, and nothing here asks for one: it is the
        # server's to compose now, off the Worktree of the Conversation the Set
        # was asked from — and this Conversation is a Draft, which has none.
        # `crates/server/tests/sets.rs` is where that composition is proved,
        # over a Conversation that has been checked out.
        detail = machine.succeed(f"curl -sf http://127.0.0.1:8422/api/ui/sets/{first}")
        assert "vm-project" in detail, "the CLI did not derive the project"
        assert BRANCH in detail, "the CLI did not derive the branch"

        answer(first, "first-response.json")
        printed = collect("first")

        assert "The first Set came back" in printed, f"the CLI printed:\n{printed}"
        assert "Posted through the same API" in printed, f"the CLI printed:\n{printed}"
        # stdout is the Response and nothing else — the agent parses it as it
        # stands, so anything the CLI had to say has to have gone to stderr. It
        # says it as a YAML comment, which is what a merged capture stays
        # parseable through; on stdout alone there is nothing to comment.
        assert "# verkstead:" not in printed, f"the CLI printed:\n{printed}"

    with subtest("a pending Set and its waiting agent survive the service restarting"):
        ask("second")
        pending = waiting("second")

        # Stopped and started rather than restarted: with the server gone for a
        # moment the agent is certain to find it missing, and can be *seen*
        # deciding to come back. A `restart` may slip between two of its polls
        # and prove nothing. `Restart=always` does not fight an explicit stop.
        machine.succeed("systemctl stop verkstead.service")
        machine.wait_until_succeeds("grep -q retrying /root/second/log")

        machine.succeed("systemctl start verkstead.service")
        machine.wait_for_open_port(8422)

        # The Set outlived the process that took it: 204 is "still pending, come
        # back", where a database that had not survived would answer 404.
        code = status_code(f"{asking_from()}/api/v1/sets/{pending}/response?hold=0")
        assert code == "204", f"the pending Set answered {code} after the restart"

        # And so did the Set that was answered before the restart.
        machine.succeed(
            f"curl -sf '{asking_from()}/api/v1/sets/{first}/response?hold=0'"
            " | grep -q 'The first Set came back'"
        )

        # And so did the repos registered before it: a registration is a thing
        # done once and expected to hold, so a service that forgot them on a
        # restart would be one nobody could rely on.
        listed = machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/repos")
        for watched in ["/srv/repos/inside", "/home/watched/inside"]:
            assert watched in listed, f"{watched} was forgotten:\n{listed}"

        # The agent did not fail when the server went away; it reconnects its
        # wait, so answering now still reaches it.
        machine.succeed("test ! -e /root/second/status")
        answer(pending, "second-response.json")
        printed = collect("second")

        assert "recovered its wait" in printed, f"the CLI printed:\n{printed}"

    with subtest("the running service makes a worktree in its data directory"):
        # Everything a grilling needs, done through the same endpoints the
        # workbench presses — so what makes the worktree is the service, from
        # inside its own hardening, and not the test reaching around it.
        #
        committed("${grillingRepo}")

        outcome = post("/api/ui/repos", {"path": "${grillingRepo}"})
        assert outcome == '"Added"', f"${grillingRepo} was answered {outcome}"

        repos = json.loads(machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/repos"))
        repo_id = next(row["id"] for row in repos if row["path"] == "${grillingRepo}")

        # The one model this account runs on, named once: it is saved with the
        # Profile and picked again with it, and the two have to agree.
        model = "claude-opus-5"

        # The account goes in the shape its own agent type keeps one, with the
        # type beside it: a Claude Profile is the pair bound over `~/.claude`
        # and `~/.claude.json`, where every other type this knows about is one
        # home. Flat on the wire, which is what `ProfileAccount` is.
        saved = post(
            "/api/ui/profiles",
            {
                "name": "vm",
                "account": {
                    "agent_type": "Claude",
                    "claude_dir": "${account}/.claude",
                    "config_file": "${account}/.claude.json",
                },
                "models": [model],
            },
        )
        assert saved == '"Saved"', f"the Profile was answered {saved}"

        profiles = json.loads(
            machine.succeed("curl -sf http://127.0.0.1:8422/api/ui/profiles")
        )
        assert len(profiles) == 1, f"expected the one Profile, got:\n{profiles}"
        profile_id = profiles[0]["id"]
        assert profiles[0]["broken"] is None, (
            "the Profile's pair is inside a Watched Path and on disk, so the "
            f"service should be able to reach it: {profiles[0]}"
        )

        started = json.loads(post("/api/ui/conversations", {"repo_id": repo_id}))
        conversation = started["Started"]["id"]

        # Every precondition `start_grilling` checks — a Brief, and all three
        # roles settled. A Pairing is a Profile and one of its models together:
        # there is no default model anywhere, so neither half is left to be
        # assumed.
        post(
            f"/api/ui/conversations/{conversation}/brief",
            {"markdown": "Whether the packaged unit can host a sandbox."},
        )

        # The two roles that can be picked away — the grilling and the review —
        # are chosen inside a wrapper naming the Pairing, because the absence of
        # one there is the picker's own "no grilling" row rather than a choice
        # nobody has made yet. The implementation has no such row: something has
        # to build the work.
        pairing = {"profile_id": profile_id, "model": model}
        for which, choice in [
            ("grilling-pairing", {"pairing": pairing}),
            ("implementation-pairing", pairing),
            ("review-pairing", {"pairing": pairing}),
        ]:
            chosen = post(
                f"/api/ui/conversations/{conversation}/{which}", choice
            )
            assert chosen == '"Chosen"', f"the {which} was answered {chosen}"

        grilling = post(f"/api/ui/conversations/{conversation}/grill", {})
        assert grilling == '"Started"', f"grilling was answered {grilling}"

        # The worktree is where the design says it is — under the State
        # Directory, which is Verkstead's own, rather than inside a Watched
        # Path. `ProtectSystem = "strict"` leaves exactly that one directory
        # writable, which is the half no crate test can ask.
        view = json.loads(
            machine.succeed(
                f"curl -sf http://127.0.0.1:8422/api/ui/conversations/{conversation}"
            )
        )
        worktree = view["worktree"]["path"]
        assert worktree.startswith("/var/lib/verkstead/worktrees/"), (
            f"the worktree is at {worktree}"
        )

        owner = machine.succeed(f"stat -c %U:%G {worktree}").strip()
        assert owner == "verkstead:verkstead", f"the worktree is owned by {owner}"

        # A checkout git made, rather than a directory that merely exists: the
        # branch is checked out in it and the repository has a record of it.
        #
        # Read off the repository rather than asked of `git worktree list`,
        # because asking would mean running git in a repository this test does
        # not own — which is the same refusal the service would meet the other
        # way round.
        machine.succeed(f"test -e {worktree}/tracked.txt")
        machine.succeed(
            f"test -d ${grillingRepo}/.git/worktrees/{worktree.rsplit('/', 1)[-1]}"
        )

    with subtest("a sandbox starts under the service's own hardening"):
        # The service's home with the machine's own credentials left lying in
        # it — a gitconfig, a gh login and a private key — none of which a
        # session has any business seeing.
        machine.succeed("mkdir -p ${home}/.config/gh")
        machine.succeed(
            "install -m 0644 /etc/verkstead-vm-test/gitconfig ${home}/.gitconfig"
        )
        machine.succeed(
            "install -m 0600 /etc/verkstead-vm-test/gh-hosts.yml"
            " ${home}/.config/gh/hosts.yml"
        )
        machine.succeed("echo 'a private key' > ${home}/.ssh-key")
        machine.succeed("chown -R verkstead:verkstead ${home}")

        # The probe unit runs `verkstead.service`'s own `serviceConfig`, so what
        # this starts is a sandbox under the hardening the server runs under. A
        # setting that stops bwrap fails here and nowhere else — the crate tests
        # run in a dev shell, which forbids none of it.
        machine.succeed("systemctl start verkstead-sandbox-probe.service")

        printed = machine.succeed(
            "journalctl -u verkstead-sandbox-probe.service --no-pager -o cat"
        )
        reported = dict(
            line.removeprefix("verkstead-probe ").split("=", 1)
            for line in printed.splitlines()
            if line.startswith("verkstead-probe ")
        )
        assert reported, f"the probe reported nothing:\n{printed}"

        # The surface, as the sandbox tests describe it — asked of the packaged
        # unit this time rather than of a dev shell.
        assert reported["worktree"] == "write", "a session commits its work"
        assert reported["git-dir"] == "write", (
            "the objects and refs a commit is written into are the Repo's"
        )
        assert reported["claude-dir"] == "write"
        assert reported["claude-config"] == "write"
        assert reported["nix"] == "read"
        assert reported["gitconfig"] == "absent", (
            "nothing of the host's git identity comes in: who a session commits as "
            "is the configured author, handed to it as GIT_CONFIG_* — which is the "
            "sandbox tests' to check, an environment variable being nothing this "
            "unit's hardening can reach"
        )
        assert reported["gh"] == "absent", (
            "and nothing of the host's gh login either: GitHub auth is the "
            "configured token, handed to a session as `GH_TOKEN`"
        )
        # The two listings come back without the separator the probe put after
        # the last entry, because the journal trims a line's trailing
        # whitespace. Nothing to work around — just what to compare against.
        assert reported["repo-holds"] == ".git", (
            "the Repo is inside as its git directory and nothing else — not the "
            f"checkout the worktree was made from: {reported['repo-holds']!r}"
        )
        assert reported["sibling"] == "absent", (
            "another repository under the same Watched Path is another Conversation's"
        )
        assert reported["home"] == ".claude .claude.json", (
            f"everything else in the service's home is absent inside: {reported['home']!r}"
        )

        # The shared build cache, which is the whole of what the module has to
        # arrange for it: a directory systemd made and the unit's hardening
        # leaves writable, and an sccache on the service's path for the server
        # to resolve and bind in. No installer action for either.
        assert reported["build-cache"] == "write", (
            "a cache a session cannot write to is no cache — `CacheDirectory` "
            "makes it and `BindPaths` says the sandbox has to see it"
        )
        assert reported["cargo-home"] == "${cacheDir}/cargo", (
            f"cargo downloads into the shared cache: {reported['cargo-home']!r}"
        )
        assert reported["wrapper"] == "/verkstead/bin/sccache", (
            "the module puts sccache on the service's path so the server "
            f"resolves one without being told: {reported['wrapper']!r}"
        )
        assert reported["sccache"] == "read", (
            "and it is bound in read-only, like the `verkstead` beside it"
        )
        assert reported["sccache-size"] == "30G", (
            f"the default nobody has been to the settings page to change: "
            f"{reported['sccache-size']!r}"
        )

        # And the operations the hardening is what would forbid: a UTS namespace
        # with a hostname of its own, a fresh `/proc` in a pid namespace of its
        # own, a `/tmp` that is the sandbox's, the host's network, and a JIT.
        assert reported["hostname"] == "verkstead", (
            f"the sandbox took the host's hostname: {reported['hostname']}"
        )
        assert reported["tmp-holds"] == "0", (
            f"`/tmp` inside is the host's, holding {reported['tmp-holds']} entries"
        )
        assert reported["network"] == "reached", (
            "the filesystem is the boundary; the network is not"
        )
        assert reported["jit"] == "ran", (
            "`claude` is node, and node aborts where it cannot make what it wrote "
            "executable"
        )
  '';
}
