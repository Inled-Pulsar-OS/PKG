import QtQuick

Item {
    id: userModelRoot

    property alias usersList: usersList
    property string lastNameUser: (typeof userModel !== "undefined" && userModel.lastUser) ? userModel.lastUser : ""
    property string finalLoginUserName: (lastNameUser && lastNameUser !== "") ? lastNameUser : ""
    property url lastUrlAvatar

    property bool fullChargeModel: customUserModel.count === userModel.count

    function updateOrdenModel(name){
        if (customUserModel.count > 0 && name && name !== "") {
            var index = 0
            for (var a = 0; a < customUserModel.count; a++){
                if (customUserModel.get(a).name === name) {
                    index = a
                    lastUrlAvatar = customUserModel.get(a).icon
                    break
                }
            }

            usersList.clear()

            for (var u = index; u < (index + customUserModel.count); u++) {
                var newIndex = u % (customUserModel.count)
                usersList.append({ name: customUserModel.get(newIndex).name, icon: customUserModel.get(newIndex).icon });
            }
        }
    }

    Repeater {
        model: userModel
        delegate: Item {
            Component.onCompleted: {
                customUserModel.append({ name: model.name, icon: model.icon });
                if (model.lastUser && model.lastUser !== "") {
                    userModelRoot.lastNameUser = model.lastUser
                    userModelRoot.finalLoginUserName = model.lastUser
                } else if (!userModelRoot.finalLoginUserName || userModelRoot.finalLoginUserName === "") {
                    userModelRoot.finalLoginUserName = model.name
                }
            }
        }
    }

    onLastNameUserChanged: {
        if (lastNameUser && lastNameUser !== "") {
            finalLoginUserName = lastNameUser
            updateOrdenModel(lastNameUser)
        } else if (customUserModel.count > 0) {
            finalLoginUserName = customUserModel.get(0).name
            updateOrdenModel(finalLoginUserName)
        }
    }

    Connections {
        target: customUserModel
        function onCountChanged() {
            if (customUserModel.count > 0) {
                var chosen = (lastNameUser && lastNameUser !== "") ? lastNameUser : customUserModel.get(0).name;
                finalLoginUserName = chosen
                updateOrdenModel(chosen)
            }
        }
    }

    ListModel {
        id: customUserModel
    }

    ListModel {
        id: usersList
    }

    Component.onCompleted: {
        var initialUser = (typeof userModel !== "undefined" && userModel.lastUser) ? userModel.lastUser : "";
        if (initialUser && initialUser !== "") {
            finalLoginUserName = initialUser;
        }
    }
}
