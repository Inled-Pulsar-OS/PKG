// Pins librepods-ctl's exit codes, since a windowed daemon answers reopen with silence and a headless one with a refusal.

#include "ipcpath.hpp"

#include <QtTest/QtTest>
#include <QLocalServer>
#include <QLocalSocket>
#include <QProcess>
#include <QTemporaryDir>

class TestCtl : public QObject
{
    Q_OBJECT

    // Room for a process spawn on a loaded box, and above the 1000 ms the CLI itself waits for a reply.
    static constexpr int kRunTimeoutMs = 5000;

private slots:
    void init()
    {
        m_dir.reset(new QTemporaryDir);
        QVERIFY(m_dir->isValid());
        m_reply.clear();
        m_closeOnRead = true;

        m_env = QProcessEnvironment::systemEnvironment();
        m_env.insert(QStringLiteral("XDG_RUNTIME_DIR"), m_dir->path());
        // socketPath() reads the same variable in this process, so both ends agree on one temp socket.
        qputenv("XDG_RUNTIME_DIR", m_dir->path().toLocal8Bit());

        m_server.reset(new QLocalServer);
        QVERIFY(m_server->listen(OpenPods::Ipc::socketPath()));
        connect(m_server.data(), &QLocalServer::newConnection, this, &TestCtl::serveOneClient);
    }

    void reopenAnsweredWithSilence_succeedsQuietly()
    {
        Run run;
        runCtl("reopen", run);

        QCOMPARE(run.exitCode, 0);
        QCOMPARE(run.err, QByteArray());
        QCOMPARE(run.out, QByteArray());
    }

    void reopenRefused_failsAndPrintsTheRefusal()
    {
        // Exactly what daemon/main.cpp writes when it has no window.
        m_reply = "error: this daemon runs headless and has no window\n";

        Run run;
        runCtl("reopen", run);

        QCOMPARE(run.exitCode, 1);
        QCOMPARE(run.err, m_reply);
        QCOMPARE(run.out, QByteArray());
    }

    void reopenLeftHanging_failsRatherThanClaimingSuccess()
    {
        // A daemon that accepts the connection and then answers nothing at all, which is not the windowed case.
        m_closeOnRead = false;

        Run run;
        runCtl("reopen", run);

        QCOMPARE(run.exitCode, 1);
        QVERIFY(run.err.startsWith("Timed out waiting for a reply to reopen"));
    }

    void statusAnswered_printsTheReplyOnStdout()
    {
        m_reply = "{\"connected\":true}\n";

        Run run;
        runCtl("status", run);

        QCOMPARE(run.exitCode, 0);
        QCOMPARE(run.out, m_reply);
        QCOMPARE(run.err, QByteArray());
    }

    void statusClosedWithoutReplying_failsRatherThanClaimingSuccess()
    {
        // The guard lets reopen alone read a silent close as a yes, and status is the case that proves the cmd half of it.
        m_closeOnRead = true;

        Run run;
        runCtl("status", run);

        QCOMPARE(run.exitCode, 1);
        QVERIFY(run.err.startsWith("Timed out waiting for a reply to status"));
    }

    void statusLeftHanging_failsAndNamesTheCommand()
    {
        m_closeOnRead = false;

        Run run;
        runCtl("status", run);

        QCOMPARE(run.exitCode, 1);
        QVERIFY(run.err.startsWith("Timed out waiting for a reply to status"));
    }

    void cleanup()
    {
        m_server.reset();
        qunsetenv("XDG_RUNTIME_DIR");
        m_dir.reset();
    }

private:
    struct Run
    {
        int exitCode = -1;
        QByteArray out;
        QByteArray err;
    };

    void serveOneClient()
    {
        QLocalSocket *client = m_server->nextPendingConnection();
        QVERIFY(client);
        connect(client, &QLocalSocket::readyRead, client, [this, client]() {
            if (!m_reply.isEmpty()) {
                client->write(m_reply);
                client->flush();
            }
            if (m_closeOnRead) {
                client->disconnectFromServer();
            }
        });
        connect(client, &QLocalSocket::disconnected, client, &QLocalSocket::deleteLater);
    }

    // Fills run rather than returning it, because the QTRY_VERIFY below expands to a bare return.
    void runCtl(const QByteArray &verb, Run &run)
    {
        QProcess proc;
        proc.setProcessEnvironment(m_env);
        proc.start(QStringLiteral(OPENPODS_CTL_BINARY), {QString::fromLatin1(verb)});
        QVERIFY(proc.waitForStarted(kRunTimeoutMs));
        // Waiting with waitForFinished would block this event loop, and the stub server would never accept.
        QTRY_VERIFY_WITH_TIMEOUT(proc.state() == QProcess::NotRunning, kRunTimeoutMs);
        run.exitCode = proc.exitCode();
        run.out = proc.readAllStandardOutput();
        run.err = proc.readAllStandardError();
    }

    QScopedPointer<QTemporaryDir> m_dir;
    QScopedPointer<QLocalServer> m_server;
    QProcessEnvironment m_env;
    QByteArray m_reply;
    bool m_closeOnRead = true;
};

QTEST_GUILESS_MAIN(TestCtl)
#include "tst_ctl.moc"
